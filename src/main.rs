mod engine;

use engine::Engine;

fn main() {
    let engine = Engine::new();
    let app = WallpaperGui;
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(Box::new(app), native_options);
    engine.run();
}
