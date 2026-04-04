use std::{
    collections::HashMap,
    sync::atomic::{AtomicBool, Ordering},
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

//
// ============================================================
// CORE TYPES
// ============================================================
//

pub type Entity = u64;

#[derive(Default, Clone, Copy)]
pub struct Transform {
    pub position: [f32; 2],
    pub scale: [f32; 2],
}

//
// ============================================================
// ECS WORLD
// ============================================================
//

pub struct World {
    next: Entity,
    transforms: HashMap<Entity, Transform>,
}

impl World {
    pub fn new() -> Self {
        Self {
            next: 1,
            transforms: HashMap::new(),
        }
    }

    pub fn spawn(&mut self, t: Transform) -> Entity {
        let id = self.next;
        self.next += 1;
        self.transforms.insert(id, t);
        id
    }
}

//
// ============================================================
// RENDER GRAPH
// ============================================================
//

pub trait RenderNode: Send {
    fn render(&mut self, ctx: &mut RenderContext);
}

pub struct RenderContext {
    pub delta: f32,
}

pub struct RenderGraph {
    nodes: Vec<Box<dyn RenderNode>>,
}

impl RenderGraph {
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    pub fn add<N: RenderNode + 'static>(&mut self, node: N) {
        self.nodes.push(Box::new(node));
    }

    pub fn execute(&mut self, ctx: &mut RenderContext) {
        for node in &mut self.nodes {
            node.render(ctx);
        }
    }
}

//
// ============================================================
// GPU RENDERER (stub backend)
// ============================================================
//

pub struct Renderer {
    frame: u64,
}

impl Renderer {
    pub fn new() -> Self {
        Self { frame: 0 }
    }

    pub fn draw(&mut self, graph: &mut RenderGraph, dt: f32) {
        let mut ctx = RenderContext { delta: dt };
        graph.execute(&mut ctx);
        self.frame += 1;
    }
}

//
// ============================================================
// WAYLAND BACKEND (stub)
// ============================================================
//

pub struct WaylandBackend;

impl WaylandBackend {
    pub fn new() -> Self {
        println!("[engine] Wayland backend initialized");
        Self
    }

    pub fn dispatch(&mut self) {
        // TODO: real Wayland event loop
    }
}

//
// ============================================================
// SUBSYSTEMS
// ============================================================
//

pub struct MediaEngine;
impl MediaEngine {
    pub fn update(&mut self) {}
}

pub struct AudioEngine {
    pub spectrum: [f32; 128],
}
impl AudioEngine {
    pub fn update(&mut self) {}
}

pub struct PhysicsEngine;
impl PhysicsEngine {
    pub fn step(&mut self, _dt: f32) {}
}

pub struct ScriptRuntime;
impl ScriptRuntime {
    pub fn update(&mut self, _dt: f32) {}
}

pub struct WebRuntime;
impl WebRuntime {
    pub fn update(&mut self) {}
}

//
// ============================================================
// PLUGINS
// ============================================================
//

pub trait Plugin: Send {
    fn update(&mut self, dt: f32);
}

pub struct PluginHost {
    plugins: Vec<Box<dyn Plugin>>,
}

impl PluginHost {
    pub fn new() -> Self {
        Self { plugins: vec![] }
    }

    pub fn update(&mut self, dt: f32) {
        for p in &mut self.plugins {
            p.update(dt);
        }
    }
}

//
// ============================================================
// ASSETS
// ============================================================
//

pub struct AssetManager;
impl AssetManager {
    pub fn poll(&mut self) {}
}

//
// ============================================================
// PERFORMANCE CONTROL
// ============================================================
//

pub enum PerformanceMode {
    Performance,
    Balanced,
    Battery,
}

pub struct PerformanceController {
    pub mode: PerformanceMode,
}

impl PerformanceController {
    pub fn update(&mut self) {}
}

//
// ============================================================
// ENGINE Ω RUNTIME
// ============================================================
//

pub struct EngineOmega {
    world: World,
    renderer: Renderer,
    wayland: WaylandBackend,
    graph: RenderGraph,

    media: MediaEngine,
    audio: AudioEngine,
    physics: PhysicsEngine,
    scripts: ScriptRuntime,
    web: WebRuntime,
    plugins: PluginHost,
    assets: AssetManager,
    perf: PerformanceController,

    running: Arc<AtomicBool>,
}

impl EngineOmega {
    pub fn new() -> Self {
        Self {
            world: World::new(),
            renderer: Renderer::new(),
            wayland: WaylandBackend::new(),
            graph: RenderGraph::new(),

            media: MediaEngine,
            audio: AudioEngine { spectrum: [0.0; 128] },
            physics: PhysicsEngine,
            scripts: ScriptRuntime,
            web: WebRuntime,
            plugins: PluginHost::new(),
            assets: AssetManager,
            perf: PerformanceController {
                mode: PerformanceMode::Balanced,
            },

            running: Arc::new(AtomicBool::new(true)),
        }
    }

    pub fn stop_handle(&self) -> Arc<AtomicBool> {
        self.running.clone()
    }

    pub fn run(mut self) {
        println!("[engine] starting main loop");

        let mut last = Instant::now();

        while self.running.load(Ordering::Relaxed) {
            let now = Instant::now();
            let dt = (now - last).as_secs_f32();
            last = now;

            self.wayland.dispatch();

            self.assets.poll();
            self.media.update();
            self.audio.update();
            self.physics.step(dt);
            self.scripts.update(dt);
            self.web.update();
            self.plugins.update(dt);
            self.perf.update();

            self.renderer.draw(&mut self.graph, dt);

            thread::sleep(Duration::from_millis(16));
        }

        println!("[engine] shutdown complete");
    }
}
