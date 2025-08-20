use std::{
    array::from_fn,
    collections::{HashMap, HashSet},
    ops::RangeInclusive,
};

use renderer::{
    ball::BallPosition,
    chunk::{Chunk, ChunkPosition, CHUNK_SIZE},
};
use shared::{
    egui::{self},
    winit::keyboard::KeyCode,
};

use crate::{
    app::{App, State},
    tiles::Tile,
};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Tool {
    BallTool(bool),
    TileTool(Tile),
}

pub struct Simulation {
    chunks: HashMap<ChunkPosition, Chunk>,
    balls: HashMap<BallPosition, bool>,
    current_tool: Tool,
    last_mouse_pos: [f32; 2],
}

impl Simulation {
    pub fn new(mouse_pos: [f32; 2]) -> Self {
        let mut s = Self {
            chunks: HashMap::new(),
            last_mouse_pos: mouse_pos,
            current_tool: Tool::TileTool(Tile::Block),
            balls: HashMap::new(),
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
        //clamping the zoom between 64 and 8
        *app.scroll_level_mut() = app
            .scroll_level()
            .clamp(-6.0 * SCROLL_SPEED, -3.0 * SCROLL_SPEED);
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

    fn get_visible_balls(&self, app: &App) -> Vec<(BallPosition, bool)> {
        let view_size = app.camera().world_viewport_size();
        let center = app.camera().pos;
        let ranges: Vec<RangeInclusive<i32>> = center
            .iter()
            .zip(view_size)
            .map(|(center, view_size)| {
                (center - view_size / 2.0).floor() as i32
                    ..=((center + view_size / 2.0).floor() as i32)
            })
            .collect();
        let mut out = vec![];
        ranges[0].clone().for_each(|x| {
            ranges[1].clone().for_each(|y| {
                let pos = [x, y];
                if let Some(on) = self.get_ball(pos) {
                    out.push((BallPosition { position: pos }, on));
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

    fn set_ball(&mut self, pos: [i32; 2], on: bool) {
        self.balls.insert(BallPosition { position: pos }, on);
    }

    fn get_ball(&self, pos: [i32; 2]) -> Option<bool> {
        self.balls.get(&BallPosition { position: pos }).copied()
    }

    fn handle_mouse(&mut self, app: &mut App) {
        if app.mouse_buttons().0 {
            if app.is_key_pressed(KeyCode::ShiftLeft) {
                self.drag_camera(app);
            } else {
                let pos = app.get_mouse_position_world();
                let w_pos = [pos[0].floor() as i32, pos[1].floor() as i32];
                match self.current_tool {
                    Tool::BallTool(on) => self.set_ball(w_pos, on),
                    Tool::TileTool(tile) => self.set_tile(w_pos, tile),
                }
            }
        }
    }

    fn sim_step(
        &mut self,
        dir: Direction,
        dont_move: &mut HashSet<[i32; 2]>,
        duplicated: &mut HashSet<[i32; 2]>,
    ) {
        let mut balls_to_update = vec![];
        let mut balls_to_remove = vec![];
        let mut balls_to_duplicate = vec![];
        self.balls.iter().for_each(|(pos, on)| {
            let tile = self.get_tile(pos.position);
            if !dont_move.contains(&pos.position)
                && match (tile, dir) {
                    (Tile::Up, Direction::Up)
                    | (Tile::Down, Direction::Down)
                    | (Tile::Left, Direction::Left)
                    | (Tile::Right, Direction::Right) => true,
                    (Tile::Filter, Direction::Left) if !on => true,
                    (Tile::Filter, Direction::Right) if *on => true,
                    (Tile::Duplicate, Direction::Left) | (Tile::Duplicate, Direction::Right) => {
                        if !duplicated.contains(&pos.position) {
                            balls_to_duplicate.push((*pos, *on));
                        }
                        true
                    }
                    (Tile::Destroy, _) => {
                        balls_to_remove.push(*pos);
                        false
                    }
                    _ => false,
                }
            {
                balls_to_update.push(pos.position);
            }
        });
        balls_to_remove.into_iter().for_each(|pos| {
            self.balls.remove(&pos);
        });
        balls_to_update.sort_by(|a, b| match dir {
            Direction::Up => a[1].cmp(&b[1]),
            Direction::Down => b[1].cmp(&a[1]),
            Direction::Left => b[0].cmp(&a[0]),
            Direction::Right => a[0].cmp(&b[0]),
        });
        let mut failed_holds = HashSet::new();
        while let Some(pos) = balls_to_update.pop() {
            println!("updating {pos:?}");
            let next_pos = BallPosition {
                position: match dir {
                    Direction::Up => [pos[0], pos[1] + 1],
                    Direction::Down => [pos[0], pos[1] - 1],
                    Direction::Left => [pos[0] - 1, pos[1]],
                    Direction::Right => [pos[0] + 1, pos[1]],
                },
            };
            if !self.balls.contains_key(&next_pos) {
                if self.get_tile(next_pos.position) != Tile::Block {
                    let ball = self
                        .balls
                        .remove(&BallPosition { position: pos })
                        .expect("we are trying to move a ball that doesn't exist");
                    self.balls.insert(next_pos, ball);
                    dont_move.insert(next_pos.position);
                    if self.get_tile(pos) == Tile::Duplicate {
                        duplicated.insert(pos);
                    }
                }
            } else if self.get_tile(next_pos.position) == Tile::Hold && !failed_holds.contains(&next_pos.position){
                balls_to_update.push(pos);
                balls_to_update.push(next_pos.position);
            } else if self.get_tile(pos) == Tile::Hold{
                failed_holds.insert(pos);
            }
        }

        balls_to_duplicate.into_iter().for_each(|(pos, on)| {
            self.balls.insert(pos, on);
        });
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl State for Simulation {
    fn update(&mut self, app: &mut crate::app::App, delta_time: f32) {
        Simulation::update_zoom(app);
        self.handle_mouse(app);

        //ending stuff
        app.set_chunk_to_draw(self.get_visible_chunks(app));
        app.set_balls_to_draw(self.get_visible_balls(app));
        self.last_mouse_pos = app.get_mouse_position_world();
    }

    fn ui(&mut self, app: &mut crate::app::App, ctx: &shared::egui::Context) {
        egui::Window::new("tile select").show(ctx, |ui| {
            [true, false].iter().for_each(|on| {
                ui.selectable_value(
                    &mut self.current_tool,
                    Tool::BallTool(*on),
                    format!("{on:?}"),
                );
            });
            (0_u8..9_u8)
                .filter_map(|val| Some(Tool::TileTool(val.try_into().ok()?)))
                .for_each(|tile| {
                    ui.selectable_value(&mut self.current_tool, tile.clone(), format!("{tile:?}"));
                });
        });
        egui::Window::new("simulate").show(ctx, |ui| {
            if ui.button("full update").clicked() {
                dbg!([
                    Direction::Up,
                    Direction::Right,
                    Direction::Left,
                    Direction::Down,
                ]
                .into_iter()
                .fold((HashSet::new(), HashSet::new()), |(mut moved, mut dup), dir| {
                    self.sim_step(dir, &mut moved, &mut dup);
                    (moved, dup) 
                }));
            }
        });
    }
}
