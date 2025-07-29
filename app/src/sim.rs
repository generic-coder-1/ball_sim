use std::collections::HashMap;

use renderer::chunk::{Chunk, ChunkPosition};
use shared::{egui, winit::keyboard::KeyCode};

use crate::app::{App, State};

pub struct Simulation {
    chunks: HashMap<ChunkPosition, Chunk>,
    last_mouse_pos: [f32; 2],
}

impl Simulation {
    pub fn new(mouse_pos: [f32; 2]) -> Self {
        Self {
            chunks: HashMap::new(),
            last_mouse_pos: mouse_pos,
        }
    }

    fn update_zoom(app: &mut App) {
        const SCROLL_SPEED: f32 = 5.0;

        let prev = app.get_mouse_position_world();
        app.camera_mut().width = 2.0_f32.powf(-app.scroll_level() / SCROLL_SPEED);
        let curr = app.get_mouse_position_world();
        let pos = &mut app.camera_mut().pos;
        pos[0] += prev[0] - curr[0];
        pos[1] += prev[1] - curr[1];
    }

    fn drag_camera(&self, app: &mut App) {
        if app.mouse_buttons().0 && app.is_key_pressed(KeyCode::ShiftLeft) {
            let curr = app.get_mouse_position_world();
            if self.last_mouse_pos != curr {
                let pos = &mut app.camera_mut().pos;
                pos[0] += self.last_mouse_pos[0] - curr[0];
                pos[1] += self.last_mouse_pos[1] - curr[1];
            }
        }
    }
}

impl State for Simulation {
    fn update(&mut self, app: &mut crate::app::App, delta_time: f32) {
        Simulation::update_zoom(app);
        self.drag_camera(app); 
        self.last_mouse_pos = app.get_mouse_position_world();
    }

    fn ui(&mut self, app: &mut crate::app::App, ctx: &shared::egui::Context) {}
}
