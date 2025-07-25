use std::{
    array::from_fn,
    collections::{HashMap, HashSet},
    f32::consts::PI,
    sync::Arc,
    time::Instant,
};

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

pub struct App {
    state: Option<State>,
    camera: CameraUniform,
    keys_down: HashSet<KeyCode>,
    last_update_time: Instant,
    last_render_time: Instant,
}

impl App {
    pub fn new() -> Self {
        Self {
            state: None,
            camera: CameraUniform::default(),
            keys_down: HashSet::new(),
            last_update_time: Instant::now(),
            last_render_time: Instant::now(),
        }
    }

    fn ui(&mut self, ctx: &Context) {
        egui::Window::new("").show(ctx, |ui| {
            ui.label(format!("{:?}", self.camera));
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

    fn update(&mut self, delta_time: f32) {}
}

impl ApplicationHandler<State> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = Window::default_attributes();

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        self.state = Some(pollster::block_on(State::new(window)).unwrap());

        self.state.as_mut().unwrap().update_chunks(vec![&Chunk {
            position: [0; 2],
            data: from_fn(|_| 3),
        }]);
    }

    #[allow(unused_mut)]
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, mut event: State) {
        self.state = Some(event);
    }

    fn new_events(&mut self, _event_loop: &ActiveEventLoop, cause: winit::event::StartCause) {
        if self.last_update_time.elapsed().as_secs_f32() > 1.0 / 60.0 {
            let delta = self.last_update_time.elapsed().as_millis();
            self.last_update_time = Instant::now();
            self.update(delta as f32);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        if self.last_update_time.elapsed().as_secs_f32() > 1.0 / 60.0 {
            let delta = self.last_update_time.elapsed().as_millis();
            self.last_update_time = Instant::now();
            self.update(delta as f32);
        }

        let mut state = match self.state.take() {
            Some(canvas) => canvas,
            None => return,
        };

        state.egui_platform.handle_event(&event);
        state.update_camera(self.camera);
        if state.egui_platform.captures_event(&event) {
            self.state = Some(state);
            return;
        }
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                state.resize(size.width, size.height);
            }
            WindowEvent::RedrawRequested => {
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
        event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
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
