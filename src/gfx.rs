use log::info;
use wgpu::util::DeviceExt;
use winit::dpi::PhysicalSize;
use winit::window::Window;

pub use wgpu::Color;

pub struct Renderer {
    size: PhysicalSize<u32>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface,
    config: wgpu::SurfaceConfiguration,
    render_pipeline: Option<wgpu::RenderPipeline>,
    v_vec: Vec<VertexRaw>,
    v_buffer: Option<wgpu::Buffer>,
}

impl Renderer {
    pub(crate) async fn new(window: &Window) -> Self {
        info!("Creating Renderer");

        let instance = wgpu::Instance::new(wgpu::Backends::all());
        let surface = unsafe { instance.create_surface(&window) };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: Default::default(),
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Main device"),
                    features: Default::default(),
                    limits: Default::default(),
                },
                None,
            )
            .await
            .unwrap();

        let size = window.inner_size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_supported_formats(&adapter)[0],
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };

        surface.configure(&device, &config);

        Self {
            size,
            device,
            queue,
            surface,
            config,
            render_pipeline: None,
            v_vec: vec![],
            v_buffer: None
        }
    }

    pub fn add_v_buffer(&mut self, buffer: Vec<Vertex>) {
        self.v_vec = buffer.iter().map(|el| el.into()).collect();
        self.v_buffer = Some(self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("v_buffer"),
            contents: bytemuck::cast_slice(&self.v_vec),
            usage: wgpu::BufferUsages::VERTEX,
        }));
    }

    pub(crate) fn init_pipeline(&mut self) {
        let render_pipeline_layout =
            self.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("render_pipeline_layout"),
                    bind_group_layouts: &[],
                    push_constant_ranges: &[],
                });

        let vertex_shader_module = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("default_vertex_shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("../res/vertex_default.wgsl").into()),
            });

        let fragment_shader_module =
            self.device
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("default_fragment_shader"),
                    source: wgpu::ShaderSource::Wgsl(
                        include_str!("../res/fragment_default.wgsl").into(),
                    ),
                });

        self.render_pipeline = Some(self.device.create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
                label: Some("default_render_pipeline"),
                layout: Some(&render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &vertex_shader_module,
                    entry_point: "vs_main",
                    buffers: &[VertexRaw::layout()],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &fragment_shader_module,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: self.config.format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::all(),
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
                    unclipped_depth: false,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
            },
        ));
    }

    pub(crate) fn render(&self) -> anyhow::Result<(), wgpu::SurfaceError> {
        let surface_texture = self.surface.get_current_texture()?;
        let view = surface_texture.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut command_encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("render_pass_encoder") });

        {
            let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("render_pass"),
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(Color {
                                r: 0.1,
                                g: 0.2,
                                b: 0.3,
                                a: 1.0,
                            }),
                            store: true
                        }
                    })
                ],
                depth_stencil_attachment: None
            });

            if let Some(pipeline) = &self.render_pipeline {
                render_pass.set_pipeline(pipeline);
                if let Some(buffer) = &self.v_buffer {
                    render_pass.set_vertex_buffer(0, buffer.slice(..));
                    render_pass.draw(0..self.v_vec.len() as u32, 0..1);
                }
            }

        }

        self.queue.submit(std::iter::once(command_encoder.finish()));
        surface_texture.present();

        Ok(())
    }

    pub(crate) fn reload_view(&mut self) {
        self.resize(self.size);
    }

    pub(crate) fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }
}

pub struct Vertex {
    pub position: cgmath::Point3<f32>,
    pub color: Color,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct VertexRaw {
    position: [f32; 3],
    color: [f32; 4],
}

impl VertexRaw {
    fn layout<'a>() -> wgpu::VertexBufferLayout<'a> {
        const ATTRIBS: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4];
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &ATTRIBS
        }
    }
}

impl From<&Vertex> for VertexRaw {
    fn from(v: &Vertex) -> Self {
        Self {
            position: [v.position.x, v.position.y, v.position.z],
            color: [
                v.color.r as f32,
                v.color.g as f32,
                v.color.b as f32,
                v.color.a as f32,
            ],
        }
    }
}
