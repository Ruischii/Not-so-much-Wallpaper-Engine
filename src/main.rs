mod engine;
mod wayland;

use engine::Engine;

fn main() {
    let engine = Engine::new();
     wayland::run_wayland_wallpaper()
        .expect("wayland failed");
    engine.run();
}
