mod engine;

use engine::Engine;

fn main() {
    let engine = Engine::new();
    engine.run();
}
