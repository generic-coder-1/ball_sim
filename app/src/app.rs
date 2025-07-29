use std::{
    array::from_fn,
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Instant,
};

use renderer::{
    chunk::{Chunk, ChunkPosition},
    state::{CameraUniform, RenderState, SurfaceError},
};
use shared::{
    egui::{self, Context},
    log,
    winit::{
        self,
        application::ApplicationHandler,
        event::{KeyEvent, WindowEvent},
        event_loop::ActiveEventLoop,
        keyboard::{KeyCode, PhysicalKey},
        window::Window,
    },
};

use crate::{tiles::Tile, LINE_HEIGHT};

pub trait State {
    fn update(&mut self, app: &mut App, delta_time: f32);
    fn ui(&mut self, app: &mut App, ctx: &Context);
}

pub struct App {
    render_state: Option<RenderState>,

    keys_down: HashSet<KeyCode>,
    mouse_position: [f32; 2],
    mouse_buttons: (bool, bool),

    camera: CameraUniform,

    scroll_level: f32,

    last_update_time: Instant,
    last_render_time: Instant,

    exiting: bool,

    state: Option<Box<dyn State>>,
}

impl App {
    pub fn new(update_loop: Option<Box<(dyn State + 'static)>>) -> Self {
        Self {
            render_state: None,
            camera: CameraUniform {
                pos: [0.0; 2],
                min_ratio: 4.0 / 3.0,
                width: 4.0,
                ..Default::default()
            },
            keys_down: HashSet::new(),
            last_update_time: Instant::now(),
            last_render_time: Instant::now(),
            mouse_position: [0.0; 2],
            mouse_buttons: (false, false),
            scroll_level: 0.0,
            exiting: false,
            state: update_loop,
        }
    }

    fn ui(&mut self, ctx: &Context) {
        egui::Window::new("").show(ctx, |ui| {
            ui.label(format!("{:?}", self.camera));
            ui.label(format!("{:?}", self.get_mouse_position_world()));
            ui.label(format!("{:?}", self.camera.world_viewport_size()));
            ui.label(format!(
                "ups: {:.2?}",
                1.0 / self.last_update_time.elapsed().as_secs_f32()
            ));
            ui.label(format!(
                "fps: {:.2?}",
                1.0 / self.last_render_time.elapsed().as_secs_f32()
            ));
        });
    }

    #[profiling::function]
    fn update(&mut self, delta_time: f32) {
        let state = self.state.take();
        if let Some(mut state) = state {
            state.update(self, delta_time);
            self.state.get_or_insert(state);
        }
    }

    pub fn in_ui(&self) -> bool {
        if let Some(state) = &self.render_state {
            state.egui_platform.context().is_pointer_over_area()
        } else {
            false
        }
    }

    fn try_update(&mut self) {
        if self.last_update_time.elapsed().as_secs_f32() > 1.0 / 60.0 {
            let delta = self.last_update_time.elapsed().as_millis();
            self.last_update_time = Instant::now();
            self.update(delta as f32);
        }
    }

    pub fn get_mouse_position_world(&self) -> [f32; 2] {
        self.camera.camera_to_world(self.mouse_position)
    }

    pub fn set_update_loop(&mut self, state: Box<dyn State>) {
        self.state = Some(state);
    }

    pub fn is_key_pressed(&self, key: KeyCode) -> bool {
        self.keys_down.contains(&key)
    }

    pub fn camera(&self) -> &CameraUniform {
        &self.camera
    }

    pub fn camera_mut(&mut self) -> &mut CameraUniform{
        &mut self.camera
    }

    pub fn mouse_buttons(&self) -> (bool, bool) {
        self.mouse_buttons
    }

    pub fn scroll_level(&self) -> f32 {
        self.scroll_level
    }

    pub fn scroll_level_mut(&mut self) -> &mut f32 {
        &mut self.scroll_level
    }
}

impl ApplicationHandler<RenderState> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = Window::default_attributes();

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        self.render_state = Some(pollster::block_on(RenderState::new(window)).unwrap());

        //default chunk
        self.render_state.as_mut().unwrap().update_chunks(
            vec![ChunkPosition { position: [0; 2] }],
            vec![Chunk {
                data: from_fn(|_| Into::<u8>::into(Tile::Flat)),
            }],
        );

        //updating camera
        let size = self.render_state.as_ref().unwrap().window.inner_size();
        self.camera.screensize = [size.width as f32, size.height as f32];
        self.render_state
            .as_mut()
            .unwrap()
            .update_camera(self.camera);
    }

    #[allow(unused_mut)]
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, mut event: RenderState) {
        self.render_state = Some(event);
    }

    fn new_events(&mut self, _event_loop: &ActiveEventLoop, _cause: winit::event::StartCause) {
        //self.try_update();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        if self.exiting {
            event_loop.exit();
            return;
        }

        self.try_update();

        let mut state = match self.render_state.take() {
            Some(canvas) => canvas,
            None => return,
        };

        state.egui_platform.handle_event(&event);
        if state.egui_platform.captures_event(&event) {
            self.render_state = Some(state);
            return;
        }
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                state.resize(size.width, size.height);
                let size = state.window.inner_size();
                self.camera.screensize = [size.width as f32, size.height as f32];
            }
            WindowEvent::RedrawRequested => {
                profiling::scope!("rendering");
                state.update_camera(self.camera);

                match state.render(|ctx| {
                    self.ui(ctx);
                    let mut state = self.state.take();
                    if let Some(ref mut state) = &mut state {
                        state.ui(self, ctx);
                    }
                    self.state = state;
                }) {
                    Ok(_) => {
                        self.last_render_time = Instant::now();
                    }
                    // Reconfigure the surface if it's lost or outdated
                    Err(SurfaceError::Lost | SurfaceError::Outdated) => {
                        let size = state.window.inner_size();
                        state.resize(size.width, size.height);
                        state.window.request_redraw();
                    }
                    Err(e) => {
                        log::error!("Unable to render {e}");
                    }
                }
            }
            WindowEvent::CursorMoved {
                device_id: _,
                position,
            } => {
                let new_pos = [position.x as f32, position.y as f32];
                self.mouse_position = new_pos;
            }
            WindowEvent::MouseInput {
                device_id: _,
                state,
                button,
            } => {
                (|| {
                    *match button {
                        winit::event::MouseButton::Left => &mut self.mouse_buttons.0,
                        winit::event::MouseButton::Right => &mut self.mouse_buttons.1,
                        _ => {
                            return;
                        }
                    } = state.is_pressed();
                })();
            }
            WindowEvent::MouseWheel {
                device_id: _,
                delta,
                phase: _,
            } => {
                self.scroll_level += match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, scroll_lines) => {
                        scroll_lines * LINE_HEIGHT
                    }
                    winit::event::MouseScrollDelta::PixelDelta(physical_position) => {
                        physical_position.y as f32
                    }
                };
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state,
                        ..
                    },
                ..
            } => match (code, state.is_pressed()) {
                (keycode, true) => self.keys_down.insert(keycode).consume(),
                (keycode, false) => self.keys_down.remove(&keycode).consume(),
            },
            _ => {}
        }
        self.render_state = Some(state);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
        if let Some(state) = self.render_state.as_mut() {
            state.window.request_redraw()
        }
    }
}

trait Consume
where
    Self: Sized,
{
    fn consume(self) {}
}

impl<T> Consume for T {}
