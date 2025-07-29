use std::{array::from_fn, collections::HashSet, sync::Arc, time::Instant};

use renderer::{
    chunk::Chunk,
    state::{CameraUniform, State, SurfaceError},
};
use shared::{
    egui::{self, Context, Slider},
    log,
    winit::{
        self,
        application::ApplicationHandler,
        event::{KeyEvent, StartCause, WindowEvent},
        event_loop::ActiveEventLoop,
        keyboard::{KeyCode, PhysicalKey},
        window::Window,
    },
};

use crate::{tiles::Tile, LINE_HEIGHT};

pub struct App {
    state: Option<State>,

    keys_down: HashSet<KeyCode>,
    mouse_position: [f32; 2],
    mouse_buttons: (bool, bool),

    camera: CameraUniform,

    zoom_speed: f32,

    last_update_time: Instant,
    last_render_time: Instant,
}

impl App {
    pub fn new() -> Self {
        Self {
            state: None,
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
            zoom_speed: 10.0,
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
    fn update(&mut self, delta_time: f32) {}

    fn try_update(&mut self) {
        if self.last_update_time.elapsed().as_secs_f32() > 1.0 / 60.0 {
            let delta = self.last_update_time.elapsed().as_millis();
            self.last_update_time = Instant::now();
            self.update(delta as f32);
        }
    }

    fn get_mouse_position_world(&self) -> [f32; 2] {
        self.camera.camera_to_world(self.mouse_position)
    }
}

impl ApplicationHandler<State> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = Window::default_attributes();

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        self.state = Some(pollster::block_on(State::new(window)).unwrap());

        //default chunk
        self.state.as_mut().unwrap().update_chunks(vec![&Chunk {
            position: [0; 2],
            data: from_fn(|_| Tile::Flat.into()),
        }]);

        //updating camera
        let size = self.state.as_ref().unwrap().window.inner_size();
        self.camera.screensize = [size.width as f32, size.height as f32];
        self.state.as_mut().unwrap().update_camera(self.camera);
    }

    #[allow(unused_mut)]
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, mut event: State) {
        self.state = Some(event);
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
        self.try_update();

        let mut state = match self.state.take() {
            Some(canvas) => canvas,
            None => return,
        };

        state.egui_platform.handle_event(&event);
        if state.egui_platform.captures_event(&event) {
            self.state = Some(state);
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
                match state.render(|ctx| self.ui(ctx)) {
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

                if !state.egui_platform.captures_event(&event) {
                    if self.mouse_buttons.0 {
                        let curr = self.mouse_position;
                        let curr_world = self.get_mouse_position_world();

                        self.mouse_position = new_pos;
                        let future_pos = self.get_mouse_position_world();

                        self.camera.pos[0] -= future_pos[0] - curr_world[0];
                        self.camera.pos[1] -= future_pos[1] - curr_world[1];
                        self.mouse_position = curr;
                    }
                }

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
                let dist = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, scroll_lines) => {
                        scroll_lines * LINE_HEIGHT
                    }
                    winit::event::MouseScrollDelta::PixelDelta(physical_position) => {
                        physical_position.y as f32
                    }
                };
                let prev = self.get_mouse_position_world();
                self.camera.width *= 2.0_f32.powf(-dist / self.zoom_speed);
                let post = self.get_mouse_position_world();
                self.camera.pos = [
                    self.camera.pos[0] + prev[0] - post[0],
                    self.camera.pos[1] + prev[1] - post[1],
                ]
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
                (KeyCode::Escape, true) => event_loop.exit(),
                (keycode, true) => self.keys_down.insert(keycode).consume(),
                (keycode, false) => self.keys_down.remove(&keycode).consume(),
            },
            _ => {}
        }
        self.state = Some(state);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
        if let Some(state) = self.state.as_mut() {
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
