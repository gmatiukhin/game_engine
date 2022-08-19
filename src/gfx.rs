use log::{info, warn};
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

    meshes: Vec<MeshRaw>,
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
            meshes: vec![],
        }
    }

    pub fn add_mesh(&mut self, mesh: &Mesh) {
        self.meshes.push(mesh.as_mesh_raw(&self.device));
    }

    pub fn update_mesh(&mut self, mesh: &mut Mesh) {
        if let Some(index) = self.meshes.iter().position(|el| el == mesh) {
            if let Some(mesh_raw) = self.meshes.get_mut(index) {
                *mesh_raw = mesh.as_mesh_raw(&self.device);
            }
        }
    }

    pub(crate) fn init(&mut self) {
        self.init_pipeline();
    }

    fn init_pipeline(&mut self) {
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
        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut command_encoder =
            self.device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("render_pass_encoder"),
                });

        {
            let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("render_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: true,
                    },
                })],
                depth_stencil_attachment: None,
            });

            if let Some(pipeline) = &self.render_pipeline {
                render_pass.set_pipeline(pipeline);
                for mesh in &self.meshes {
                    mesh.render(&mut render_pass);
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
        const ATTRIBS: [wgpu::VertexAttribute; 2] =
            wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4];
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &ATTRIBS,
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

pub struct Mesh {
    pub name: String,

    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

impl Mesh {
    fn as_mesh_raw(&self, device: &wgpu::Device) -> MeshRaw {
        info!("Transforming Mesh into MeshRaw");
        let mut v_buffer: Option<wgpu::Buffer> = None;
        if !self.vertices.is_empty() {
            let v_vec_raw: Vec<VertexRaw> = self.vertices.iter().map(|el| el.into()).collect();
            v_buffer = Some(
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("{}'s vertex buffer", self.name)),
                    contents: bytemuck::cast_slice(&v_vec_raw),
                    usage: wgpu::BufferUsages::VERTEX,
                }),
            );
        } else {
            warn!("Empty vertex buffer of {}", self.name);
        }

        let mut i_buffer: Option<wgpu::Buffer> = None;
        if !self.indices.is_empty() {
            i_buffer = Some(
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("{}'s index buffer", self.name)),
                    contents: bytemuck::cast_slice(&self.indices),
                    usage: wgpu::BufferUsages::INDEX,
                }),
            );
        } else {
            warn!("Empty index buffer of {}", self.name);
        }

        MeshRaw {
            name: self.name.clone(),
            vertex_buffer: v_buffer,
            vertices_length: self.vertices.len() as u32,
            index_buffer: i_buffer,
            indices_length: self.indices.len() as u32,
        }
    }
}

struct MeshRaw {
    name: String,

    vertex_buffer: Option<wgpu::Buffer>,
    vertices_length: u32,

    index_buffer: Option<wgpu::Buffer>,
    indices_length: u32,
}

impl MeshRaw {
    fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        if let Some(v_buffer) = &self.vertex_buffer {
            render_pass.set_vertex_buffer(0, v_buffer.slice(..));
        }

        if let Some(i_buffer) = &self.index_buffer {
            render_pass.set_index_buffer(i_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..self.indices_length, 0, 0..1);
        } else {
            render_pass.draw(0..self.vertices_length, 0..1);
        }
    }
}

impl PartialEq<Mesh> for MeshRaw {
    fn eq(&self, other: &Mesh) -> bool {
        self.name == other.name
    }
}
