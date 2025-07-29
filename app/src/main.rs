use std::env;

use app::App;
use shared::{anyhow, env_logger, winit::event_loop::EventLoop};
use sim::Simulation;

mod app;
mod tiles;
mod sim;
pub const LINE_HEIGHT: f32 = 1.;

fn main() {
    env::set_var("RUST_BACKTRACE", "1");
    run().unwrap()
}

pub fn run() -> anyhow::Result<()> {
    env_logger::init();
    let event_loop = EventLoop::with_user_event().build()?;
    let mut app = App::new(None);
    app.set_update_loop(Box::new(Simulation::new(app.get_mouse_position_world())));
    event_loop.run_app(&mut app)?;

    Ok(())
}
