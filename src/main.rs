mod engine;
mod wayland;

use engine::Engine;
use anyhow::Result;

fn main() -> Result<()> {
    let engine = Engine::new();

    // run engine WITH wallpaper browser UI
    engine.run_with_ui();

    Ok(())
}
