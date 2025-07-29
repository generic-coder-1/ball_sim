use std::env;

use app::App;
use shared::{anyhow, env_logger, winit::event_loop::EventLoop};

mod app;
mod tiles;
pub const LINE_HEIGHT: f32 = 1.;

fn main() {
    env::set_var("RUST_BACKTRACE", "1");
    run().unwrap()
}

pub fn run() -> anyhow::Result<()> {
    env_logger::init();
    let event_loop = EventLoop::with_user_event().build()?;
    let mut app = App::new();
    event_loop.run_app(&mut app)?;

    Ok(())
}
