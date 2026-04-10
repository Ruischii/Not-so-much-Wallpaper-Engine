// src/engine/core.rs
use std::{
    collections::HashMap,
    sync::atomic::{AtomicBool, Ordering},
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

use super::{
    gpu::Renderer,
    media::MediaEngine,
    render::RenderGraph,
    ui::{run_ui, UiCommand},
    videonode::VideoNode,        // ← Added import for VideoNode
    wayland::WaylandBackend,
};

// ====================== CORE TYPES ======================
pub type Entity = u64;

#[derive(Default, Clone, Copy)]
pub struct Transform {
    pub position: [f32; 2],
    pub scale: [f32; 2],
}

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

// ====================== SMALL SUBSYSTEMS ======================
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

pub struct AssetManager;
impl AssetManager {
    pub fn poll(&mut self) {}
}

#[derive(PartialEq)]
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

// ====================== MAIN ENGINE ======================
pub struct Engine {
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

impl Engine {
    pub fn new() -> Self {
        let graph = RenderGraph::new();

        // TODO: Create VideoNode properly when you have WlShm and QueueHandle
        // For now we leave the graph empty. You can add VideoNode later.

        Self {
            world: World::new(),
            renderer: Renderer::new(),
            wayland: WaylandBackend::new(),
            graph,

            media: MediaEngine::new(),
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

    pub fn run_with_ui(mut self) {
        use crossbeam_channel::unbounded;

        let (tx, rx) = unbounded();

        run_ui(tx.clone());

        println!("[engine] starting main loop + UI");

        let mut last = Instant::now();

        while self.running.load(Ordering::Relaxed) {
            // Handle UI commands
            while let Ok(cmd) = rx.try_recv() {
                match cmd {
                    UiCommand::Quit => {
                        self.running.store(false, Ordering::Relaxed);
                    }
                    _ => {}
                }
            }

            let now = Instant::now();
            let dt = (now - last).as_secs_f32();
            last = now;

            self.wayland.dispatch();
            self.media.update(dt);

            // VIDEO PIPELINE
            if let Some(frame) = self.media.take_frame() {
                self.renderer.submit_video_frame(&frame);

                // Try to submit frame to VideoNode if it exists in the graph
                if let Some(node) = self.graph.first_mut() {
                    if let Some(video) = node.downcast_mut::<VideoNode>() {
                        video.submit_frame(frame);
                    }
                }
            }

            self.renderer.draw(&mut self.graph, dt);

            thread::sleep(Duration::from_millis(16));
        }

        println!("[engine] shutdown complete");
    }
}
