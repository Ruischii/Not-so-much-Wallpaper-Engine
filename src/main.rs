mod engine;
mod wayland;
mod wallpaper;

use engine::Engine;

fn main() {
    let engine = Engine::new();
      wallpaper::run().unwrap();
     wayland::run_wayland_wallpaper()
        .expect("wayland failed");
    engine.run();
}
