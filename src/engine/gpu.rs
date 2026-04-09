// src/engine/gpu.rs
use super::media::VideoFrame;

pub struct Renderer {
    frame: u64,
    pub gpu: GpuContext,
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

    pub fn draw(&mut self, graph: &mut super::render::RenderGraph, dt: f32) {
        let mut ctx = super::render::RenderContext { delta: dt };
        graph.execute(&mut ctx);

        if let Some(_view) = &self.gpu.texture_view {
            let encoder = self.gpu.device.create_command_encoder(
                &wgpu::CommandEncoderDescriptor {
                    label: Some("render-encoder"),
                },
            );
            self.gpu.queue.submit(Some(encoder.finish()));
        }

        self.frame += 1;
    }
}

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
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
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
