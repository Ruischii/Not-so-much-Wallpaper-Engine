// src/engine/gpu.rs
use super::media::VideoFrame;
use std::path::PathBuf;

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

    pub fn load_texture(&mut self, path: PathBuf) {
        println!("[gpu] loading image: {:?}", path);

        let img = match image::open(&path) {
            Ok(i) => i.to_rgba8(),
            Err(e) => {
                eprintln!("[gpu] failed to load image: {:?}", e);
                return;
            }
        };

        let (w, h) = img.dimensions();
        self.gpu.upload_image(&img, w, h);
    }

    pub fn submit_video_frame(&mut self, frame: &VideoFrame) {
        self.gpu.upload_frame(frame);
    }

    pub fn draw(&mut self, graph: &mut super::render::RenderGraph, dt: f32) {
        let mut ctx = super::render::RenderContext { delta: dt };
        graph.execute(&mut ctx);

        self.gpu.render(); // 🔥 real render

        self.frame += 1;
    }
}

// ====================== GPU CONTEXT ======================

pub struct GpuContext {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,

    pub texture: Option<wgpu::Texture>,
    pub texture_view: Option<wgpu::TextureView>,
    pub size: (u32, u32),

    sampler: wgpu::Sampler,
    bind_group: Option<wgpu::BindGroup>,
    pipeline: wgpu::RenderPipeline,
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
                &wgpu::DeviceDescriptor::default(),
                None,
            ),
        )
        .expect("Failed to create device");

        // ================= SHADER =================
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());

        let bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("texture-layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(
                            wgpu::SamplerBindingType::Filtering
                        ),
                        count: None,
                    },
                ],
            });

        let pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("pipeline-layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::TextureFormat::Bgra8UnormSrgb.into())],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview: None,
        });

        Self {
            device,
            queue,
            texture: None,
            texture_view: None,
            size: (0, 0),
            sampler,
            bind_group: None,
            pipeline,
        }
    }

    fn ensure_texture(&mut self, width: u32, height: u32) {
        if self.size == (width, height) && self.texture.is_some() {
            return;
        }

        println!("[wgpu] creating texture {}x{}", width, height);

        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("gpu-texture"),
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

    pub fn upload_image(&mut self, img: &image::RgbaImage, width: u32, height: u32) {
        self.ensure_texture(width, height);

        let texture = self.texture.as_ref().unwrap();

        self.queue.write_texture(
            texture.as_image_copy(),
            img,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        // 🔥 CREATE BIND GROUP
        let layout = self.pipeline.get_bind_group_layout(0);

        self.bind_group = Some(
            self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(
                            self.texture_view.as_ref().unwrap(),
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
                label: Some("wallpaper-bind-group"),
            }),
        );

        println!("[gpu] uploaded image {}x{}", width, height);
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

    // ====================== REAL RENDER ======================
    pub fn render(&mut self) {
        if self.texture_view.is_none() || self.bind_group.is_none() {
            return;
        }

        let target = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("framebuffer"),
            size: wgpu::Extent3d {
                width: self.size.0.max(1),
                height: self.size.1.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        let view = target.create_view(&Default::default());

        let mut encoder = self.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor { label: Some("encoder") }
        );

        {
            let mut rpass = encoder.begin_render_pass(
                &wgpu::RenderPassDescriptor {
                    label: Some("render-pass"),
                    color_attachments: &[Some(
                        wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                 load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                 store: wgpu::StoreOp::Store,

                            },
                        }
                    )],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                }
            );

            rpass.set_pipeline(&self.pipeline);
            rpass.set_bind_group(0, self.bind_group.as_ref().unwrap(), &[]);
            rpass.draw(0..3, 0..1);
        }

        self.queue.submit(Some(encoder.finish()));
    }
}
