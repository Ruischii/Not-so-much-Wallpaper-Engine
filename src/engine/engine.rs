use std::{
    collections::HashMap,
    sync::atomic::{AtomicBool, Ordering},
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

use crate::wayland::wallpaper::{self, App};
use anyhow::Result;
use memmap2::MmapMut;
use wayland_client::{
    protocol::{wl_buffer::WlBuffer, wl_shm::WlShm},
    QueueHandle,
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
// VIDEO FRAME
// ============================================================
//

pub struct VideoFrame {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
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
// GPU RENDERER (stub)
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
// WAYLAND BACKEND
// ============================================================
//

#[derive(Clone)]
pub struct WaylandBackend;

impl WaylandBackend {
    pub fn new() -> Self {
        println!("[engine] Wayland backend initialized");
        Self
    }

    pub fn dispatch(&mut self) {}

    pub fn create_buffer(
        &self,
        shm: &WlShm,
        width: u32,
        height: u32,
        qh: &QueueHandle<App>,
    ) -> Result<(WlBuffer, MmapMut)> {
        wallpaper::create_buffer(shm, width, height, qh)
    }
}

//
// ============================================================
// MEDIA ENGINE (VIDEO SOURCE)
// ============================================================
//

pub struct MediaEngine {
    latest_frame: Option<VideoFrame>,
    time: f32,
}

impl MediaEngine {
    pub fn new() -> Self {
        Self {
            latest_frame: None,
            time: 0.0,
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.time += dt;

        // demo animated frame (acts like compositor video)
        let w = 640;
        let h = 360;

        let mut pixels = vec![0u8; (w * h * 4) as usize];

        for y in 0..h {
            for x in 0..w {
                let i = ((y * w + x) * 4) as usize;
                pixels[i] = ((x as f32 + self.time * 60.0) as u8);
                pixels[i + 1] = ((y as f32 + self.time * 40.0) as u8);
                pixels[i + 2] = 180;
                pixels[i + 3] = 255;
            }
        }

        self.latest_frame = Some(VideoFrame {
            pixels,
            width: w,
            height: h,
        });
    }

    pub fn take_frame(&mut self) -> Option<VideoFrame> {
        self.latest_frame.take()
    }
}

//
// ============================================================
// VIDEO RENDER NODE
// ============================================================
//

pub struct VideoNode {
    backend: WaylandBackend,
    shm: WlShm,
    qh: QueueHandle<App>,

    buffer: Option<WlBuffer>,
    mmap: Option<MmapMut>,

    width: u32,
    height: u32,
}

impl VideoNode {
    pub fn new(
        backend: WaylandBackend,
        shm: WlShm,
        qh: QueueHandle<App>,
    ) -> Self {
        Self {
            backend,
            shm,
            qh,
            buffer: None,
            mmap: None,
            width: 0,
            height: 0,
        }
    }

    fn ensure_buffer(&mut self, w: u32, h: u32) {
        if self.buffer.is_some() && self.width == w && self.height == h {
            return;
        }

        if let Ok((buf, mmap)) =
            self.backend.create_buffer(&self.shm, w, h, &self.qh)
        {
            self.buffer = Some(buf);
            self.mmap = Some(mmap);
            self.width = w;
            self.height = h;
        }
    }

    pub fn submit_frame(&mut self, frame: VideoFrame) {
        self.ensure_buffer(frame.width, frame.height);

        if let Some(mem) = &mut self.mmap {
            mem[..frame.pixels.len()].copy_from_slice(&frame.pixels);
        }
    }
}

impl RenderNode for VideoNode {
    fn render(&mut self, _ctx: &mut RenderContext) {
        // TODO:
        // wl_surface.attach(buffer)
        // wl_surface.damage_buffer(...)
        // wl_surface.commit()
    }
}

//
// ============================================================
// OTHER SUBSYSTEMS (unchanged)
// ============================================================
//

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
// ENGINE RUNTIME
// ============================================================
//

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
        Self {
            world: World::new(),
            renderer: Renderer::new(),
            wayland: WaylandBackend::new(),
            graph: RenderGraph::new(),

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

    pub fn run(mut self) {
        println!("[engine] starting main loop");

        let mut last = Instant::now();

        while self.running.load(Ordering::Relaxed) {
            let now = Instant::now();
            let dt = (now - last).as_secs_f32();
            last = now;

            self.wayland.dispatch();

            self.assets.poll();
            self.media.update(dt);
            self.audio.update();
            self.physics.step(dt);
            self.scripts.update(dt);
            self.web.update();
            self.plugins.update(dt);
            self.perf.update();

            // video frame produced here
            let _frame = self.media.take_frame();

            self.renderer.draw(&mut self.graph, dt);

            thread::sleep(Duration::from_millis(16));
        }

        println!("[engine] shutdown complete");
    }
}
