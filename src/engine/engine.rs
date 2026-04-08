use std::{
    collections::HashMap,
    sync::atomic::{AtomicBool, Ordering},
    sync::Arc,
    thread,
    time::{Duration, Instant},
    process::Command,
    fs,
    path::PathBuf,
    env,
};

use std::any::Any;

use crate::wayland::wallpaper::{self, App};
use anyhow::Result;
use memmap2::MmapMut;
use wayland_client::{
    protocol::{wl_buffer::WlBuffer, wl_shm::WlShm},
    QueueHandle,
};
// CORE TYPES
pub type Entity = u64;

#[derive(Default, Clone, Copy)]
pub struct Transform {
    pub position: [f32; 2],
    pub scale: [f32; 2],
}
// VIDEO FRAME
pub struct VideoFrame {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
}
// ECS WORLD
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
// RENDER GRAPH
pub trait RenderNode: Send + Any {
    fn render(&mut self, ctx: &mut RenderContext);
    fn as_any_mut(&mut self) -> &mut dyn Any;
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

    pub fn first_mut(
        &mut self,
    ) -> Option<&mut Box<dyn RenderNode>> {
        self.nodes.get_mut(0)
    }
}

impl dyn RenderNode {
    pub fn downcast_mut<T: RenderNode + 'static>(
        &mut self,
    ) -> Option<&mut T> {
        self.as_any_mut().downcast_mut::<T>()
    }
}
// GPU RENDERER
pub struct Renderer {
    frame: u64,
    gpu: GpuContext,
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            frame: 0,
            gpu: GpuContext::new(),
        }
    }

    pub fn submit_video_frame(&mut self, frame: &VideoFrame) {
        self.gpu.upload_frame(frame);
    }

    pub fn draw(&mut self, graph: &mut RenderGraph, dt: f32) {
        let mut ctx = RenderContext { delta: dt };

        graph.execute(&mut ctx);

        // GPU placeholder work
        if let Some(_view) = &self.gpu.texture_view {
            let encoder =
                self.gpu.device.create_command_encoder(
                    &wgpu::CommandEncoderDescriptor {
                        label: Some("render-encoder"),
                    },
                );

            self.gpu.queue.submit(Some(encoder.finish()));
        }

        self.frame += 1;
    }
}
// WGPU
pub struct GpuContext {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub texture: Option<wgpu::Texture>,
    pub texture_view: Option<wgpu::TextureView>,
    pub size: (u32, u32),
}

impl GpuContext {
    pub fn new() -> Self {
        println!("[wgpu] initializing GPU");

        let instance = wgpu::Instance::default();

        let adapter = pollster::block_on(instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            },
        ))
        .expect("No GPU adapter found");

        let (device, queue) = pollster::block_on(
            adapter.request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("engine-device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                },
                None,
            ),
        )
        .expect("Failed to create device");

        Self {
            device,
            queue,
            texture: None,
            texture_view: None,
            size: (0, 0),
        }
    }

    fn ensure_texture(&mut self, width: u32, height: u32) {
        if self.size == (width, height) && self.texture.is_some() {
            return;
        }

        println!("[wgpu] creating texture {}x{}", width, height);

        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("video-texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        let view = texture.create_view(&Default::default());

        self.texture = Some(texture);
        self.texture_view = Some(view);
        self.size = (width, height);
    }

    pub fn upload_frame(&mut self, frame: &VideoFrame) {
        self.ensure_texture(frame.width, frame.height);

        let texture = self.texture.as_ref().unwrap();

        self.queue.write_texture(
            texture.as_image_copy(),
            &frame.pixels,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * frame.width),
                rows_per_image: Some(frame.height),
            },
            wgpu::Extent3d {
                width: frame.width,
                height: frame.height,
                depth_or_array_layers: 1,
            },
        );
    }
}
// WAYLAND BACKEND
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
// SIMPLE HASH FUNCTION
fn simple_hash(input: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher =
        std::collections::hash_map::DefaultHasher::new();
    input.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}
// CACHE DIRECTORY
fn get_cache_dir() -> PathBuf {
    if let Ok(cache_home) = env::var("XDG_CACHE_HOME") {
        return PathBuf::from(cache_home)
            .join("web-wallpapers");
    }

    if let Ok(home) = env::var("HOME") {
        return PathBuf::from(home)
            .join(".cache")
            .join("web-wallpapers");
    }

    PathBuf::from("/tmp").join("web-wallpapers")
}
// WEB WALLPAPER
pub struct WebWallpaper {
    pub url: String,
    pub width: u32,
    pub height: u32,
    pub framerate: u32,
    pub is_playing: bool,
    pub cache_path: PathBuf,
}

impl WebWallpaper {
    pub fn new(url: String, width: u32, height: u32) -> Self {
        let cache_dir = get_cache_dir();
        fs::create_dir_all(&cache_dir).ok();

        let hash = simple_hash(&url);
        let cache_path = cache_dir.join(format!("{hash}.mp4"));

        Self {
            url,
            width,
            height,
            framerate: 30,
            is_playing: true,
            cache_path,
        }
    }

    pub fn download(&self) -> Result<PathBuf> {
        if self.cache_path.exists() {
            return Ok(self.cache_path.clone());
        }

        let output = Command::new("curl")
            .arg("-L")
            .arg("-o")
            .arg(&self.cache_path)
            .arg(&self.url)
            .output()?;

        if output.status.success() {
            Ok(self.cache_path.clone())
        } else {
            anyhow::bail!("download failed")
        }
    }
}
// COMPOSITOR DETECTION
#[derive(Debug, PartialEq)]
pub enum CompositorType {
    Hyprland,
    Niri,
    Sway,
    River,
    Other,
}
// WEB WALLPAPER ENGINE
pub struct WebWallpaperEngine {
    wallpapers: HashMap<String, WebWallpaper>,
    active_wallpaper: Option<String>,
    compositor_type: CompositorType,
}

impl WebWallpaperEngine {
    pub fn new() -> Self {
        Self {
            wallpapers: HashMap::new(),
            active_wallpaper: None,
            compositor_type: CompositorType::Other,
        }
    }

    pub fn add_wallpaper(
        &mut self,
        url: String,
        width: u32,
        height: u32,
    ) {
        self.wallpapers
            .insert(url.clone(), WebWallpaper::new(url, width, height));
    }

    pub fn set_active(&mut self, url: &str) -> Result<()> {
        if !self.wallpapers.contains_key(url) {
            anyhow::bail!("Wallpaper not found");
        }

        self.active_wallpaper = Some(url.to_string());
        Ok(())
    }

    pub fn update(&mut self) {}
}
// MEDIA ENGINE
pub struct MediaEngine {
    latest_frame: Option<VideoFrame>,
    time: f32,
    web_wallpapers: WebWallpaperEngine,
}

impl MediaEngine {
    pub fn new() -> Self {
        Self {
            latest_frame: None,
            time: 0.0,
            web_wallpapers: WebWallpaperEngine::new(),
        }
    }

    pub fn update(&mut self, dt: f32) {
        self.time += dt;
        self.web_wallpapers.update();

        // animated demo frame (solid animated color)
        let w = 640;
        let h = 360;

        let mut pixels =
            vec![0u8; (w * h * 4) as usize];

        for y in 0..h {
            for x in 0..w {
                let i = ((y * w + x) * 4) as usize;

                pixels[i] =
                    (x as f32 + self.time * 60.0) as u8;
                pixels[i + 1] =
                    (y as f32 + self.time * 40.0) as u8;
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
// VIDEO NODE
pub struct VideoNode {
    backend: WaylandBackend,
    shm: WlShm,
    qh: QueueHandle<App>,

    buffer: Option<WlBuffer>,
    mmap: Option<MmapMut>,

    width: u32,
    height: u32,

    // ✅ FIX ADDED
    pending_frame: Option<VideoFrame>,
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
            pending_frame: None,
        }
    }

    fn ensure_buffer(&mut self, w: u32, h: u32) {
        if self.buffer.is_some()
            && self.width == w
            && self.height == h
        {
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
        self.pending_frame = Some(frame);
    }
}

impl RenderNode for VideoNode {
    fn render(&mut self, _ctx: &mut RenderContext) {

        // upload frame
        if let Some(frame) = self.pending_frame.take() {
            self.ensure_buffer(frame.width, frame.height);

            if let Some(mem) = &mut self.mmap {
                mem[..frame.pixels.len()]
                    .copy_from_slice(&frame.pixels);
            }
        }

        // (Wayland surface attach handled elsewhere)
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
// OTHER SUBSYSTEMS
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
// ENGINE
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
            // VIDEO PIPELINE
            if let Some(frame) = self.media.take_frame() {

                // GPU upload
                self.renderer.submit_video_frame(&frame);

                // RenderGraph injection
                if let Some(node) = self.graph.first_mut() {
                    if let Some(video) =
                        node.downcast_mut::<VideoNode>()
                    {
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
// UI COMMANDS
use crossbeam_channel::{unbounded, Sender, Receiver};

#[derive(Clone)]
pub enum UiCommand {
    Play,
    Pause,
    Quit,
    LoadWallpaper(String),
}
// WALLPAPER ENTRY
#[derive(Clone)]
pub struct WallpaperEntry {
    pub name: String,
    pub path: String,
}

//
// ============================================================
// ENGINE UI
// ============================================================
//

pub struct EngineUI {
    sender: Sender<UiCommand>,
}

impl EngineUI {
    pub fn new(sender: Sender<UiCommand>) -> Self {
        Self { sender }
    }
}

impl eframe::App for EngineUI {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {

        egui::CentralPanel::default().show(ctx, |ui| {

            ui.heading("Not-so-much Wallpaper Engine");

            if ui.button("▶ Play").clicked() {
                let _ = self.sender.send(UiCommand::Play);
            }

            if ui.button("⏸ Pause").clicked() {
                let _ = self.sender.send(UiCommand::Pause);
            }

            if ui.button("❌ Quit").clicked() {
                let _ = self.sender.send(UiCommand::Quit);
            }
        });
    }
}
// UI THREAD
pub fn start_ui_thread(sender: Sender<UiCommand>) {
    let options = eframe::NativeOptions::default();

    let _ = eframe::run_native(
        "Not-so-much Wallpaper Engine",
        options,
        Box::new(|_| Box::new(EngineUI::new(sender))),
    );
}
// UI LOOP
impl Engine {

    pub fn run_with_ui(mut self) {

        let (tx, rx): (Sender<UiCommand>, Receiver<UiCommand>) =
            unbounded();

        start_ui_thread(tx.clone());

        println!("[engine] starting main loop + UI");

        let mut last = Instant::now();

        while self.running.load(Ordering::Relaxed) {

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

            if let Some(frame) = self.media.take_frame() {

                self.renderer.submit_video_frame(&frame);

                if let Some(node) = self.graph.first_mut() {
                    if let Some(video) =
                        node.downcast_mut::<VideoNode>()
                    {
                        video.submit_frame(frame);
                    }
                }
            }

            self.renderer.draw(&mut self.graph, dt);

            thread::sleep(Duration::from_millis(16));
        }
    }
}
