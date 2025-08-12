use std::{
    array::from_fn,
    collections::HashMap,
    ops::{Range, RangeInclusive},
};

use renderer::{ball::BallPosition, chunk::{Chunk, ChunkPosition, CHUNK_SIZE}};
use shared::{egui, winit::keyboard::KeyCode};

use crate::{
    app::{App, State},
    tiles::Tile,
};

pub struct Simulation {
    chunks: HashMap<ChunkPosition, Chunk>,
    current_tool: Tile,
    last_mouse_pos: [f32; 2],
}

impl Simulation {
    pub fn new(mouse_pos: [f32; 2]) -> Self {
        let mut s = Self {
            chunks: HashMap::new(),
            last_mouse_pos: mouse_pos,
            current_tool: Tile::Block,
        };
        s.chunks.insert(
            ChunkPosition { position: [0; 2] },
            Chunk {
                data: from_fn(|_| Into::<u8>::into(Tile::Down)),
            },
        );
        s
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
        let curr = app.get_mouse_position_world();
        if self.last_mouse_pos != curr {
            let pos = &mut app.camera_mut().pos;
            pos[0] += self.last_mouse_pos[0] - curr[0];
            pos[1] += self.last_mouse_pos[1] - curr[1];
        }
    }

    fn get_visible_chunks(&self, app: &App) -> Vec<(ChunkPosition, Chunk)> {
        let view_size = app.camera().world_viewport_size();
        let center = app.camera().pos;
        let ranges: Vec<RangeInclusive<i32>> = center
            .iter()
            .zip(view_size)
            .map(|(center, view_size)| {
                ((center - view_size / 2.0) / CHUNK_SIZE as f32).floor() as i32
                    ..=(((center + view_size / 2.0) / CHUNK_SIZE as f32).floor() as i32)
            })
            .collect();
        let mut out = vec![];
        ranges[0].clone().for_each(|x| {
            ranges[1].clone().for_each(|y| {
                let pos = ChunkPosition { position: [x, y] };
                if let Some(chunk) = self.chunks.get(&pos) {
                    out.push((pos, *chunk));
                }
            });
        });
        out
    }

    fn set_tile(&mut self, pos: [i32; 2], tile: Tile) {
        self.chunks
            .entry(ChunkPosition {
                position: [
                    pos[0].div_euclid(CHUNK_SIZE as i32),
                    pos[1].div_euclid(CHUNK_SIZE as i32),
                ],
            })
            .or_insert(Chunk {
                data: from_fn(|_| u8::from(Tile::Down)),
            })
            .set_tile(
                [
                    pos[0].rem_euclid(CHUNK_SIZE as i32) as u32,
                    pos[1].rem_euclid(CHUNK_SIZE as i32) as u32,
                ],
                u8::from(tile),
            );
    }

    fn get_tile(&self, pos: [i32; 2]) -> Tile {
        self.chunks
            .get(&ChunkPosition {
                position: [
                    pos[0].div_euclid(CHUNK_SIZE as i32),
                    pos[1].div_euclid(CHUNK_SIZE as i32),
                ],
            })
            .and_then(|chunk| {
                chunk
                    .get_tile([
                        pos[0].rem_euclid(CHUNK_SIZE as i32) as u32,
                        pos[1].rem_euclid(CHUNK_SIZE as i32) as u32,
                    ])
                    .try_into()
                    .ok()
            })
            .unwrap_or(Tile::Down)
    }

    fn handle_mouse(&mut self, app: &mut App) {
        if app.mouse_buttons().0 {
            if app.is_key_pressed(KeyCode::ShiftLeft) {
                self.drag_camera(app);
            } else {
                let pos = app.get_mouse_position_world();
                self.set_tile(
                    [pos[0].floor() as i32, pos[1].floor() as i32],
                    self.current_tool,
                );
            }
        }
    }
}

impl State for Simulation {
    fn update(&mut self, app: &mut crate::app::App, delta_time: f32) {
        Simulation::update_zoom(app);
        self.handle_mouse(app);

        //ending stuff
        app.set_chunk_to_draw(self.get_visible_chunks(app));
        self.last_mouse_pos = app.get_mouse_position_world();
    }

    fn ui(&mut self, app: &mut crate::app::App, ctx: &shared::egui::Context) {
        egui::Window::new("tile select").show(ctx, |ui| {
            (0_u8..9_u8).filter_map(|val|val.try_into().ok()).for_each(|tile|{
                ui.selectable_value(&mut self.current_tool, tile, format!("{tile:?}"));
            });
        });
    }
}
