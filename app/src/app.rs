use std::{collections::HashMap, f32::consts::PI, sync::Arc, time::Instant};

use renderer::state::{CameraUniform, State, SurfaceError};
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
    camera:CameraUniform,
    keys_down: HashMap<KeyCode, bool>,
    last_update_time: Instant,
    velocity: [f32; 2],
}

impl App {
    pub fn new() -> Self {
        Self {
            state: None,
            camera: CameraUniform::default(),
            keys_down: HashMap::new(),
            last_update_time: Instant::now(),
            velocity: [0.0;2],
        } 
    }

    fn ui(&mut self, ctx: &Context) {
        egui::Window::new("").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add(Slider::new(&mut self.camera.pos[1], 0.0..=10.0_f32));
                ui.label("height");
            });
            ui.horizontal(|ui| {
                ui.drag_angle(&mut self.camera.rotation);
                self.camera.rotation = self.camera.rotation.rem_euclid(PI*2.0);
                ui.label("rotation");
            });
            ui.label(format!("{:?}", self.camera))
        });
    }

    fn update(&mut self, delta_time: f32){
        //self.velocity[0] +=   
    }
}

impl ApplicationHandler<State> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = Window::default_attributes();

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        self.state = Some(pollster::block_on(State::new(window)).unwrap());
    }

    #[allow(unused_mut)]
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, mut event: State) {
        self.state = Some(event);
    }

    fn new_events(&mut self, event_loop: &ActiveEventLoop, cause: winit::event::StartCause) {
        if cause == StartCause::Poll{
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
            WindowEvent::Resized(size) => state.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                match state.render(|ctx| self.ui(ctx)) {
                    Ok(_) => {}
                    // Reconfigure the surface if it's lost or outdated
                    Err(SurfaceError::Lost | SurfaceError::Outdated) => {
                        let size = state.window.inner_size();
                        state.resize(size.width, size.height);
                    }
                    Err(e) => {
                        log::error!("Unable to render {}", e);
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
                _ => {}
            },
            _ => {}
        }
        self.state = Some(state);
    }
}
