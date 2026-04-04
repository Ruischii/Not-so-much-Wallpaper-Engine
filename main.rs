mod engine;

use engine::EngineOmega;

fn main() {
    let engine = EngineOmega::new();
    engine.run();
}
