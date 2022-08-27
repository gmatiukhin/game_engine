use wgpu::util::DeviceExt;

pub struct GUIRenderer {
    render_pipeline: wgpu::RenderPipeline,
    panels: Vec<GUIPanelBuffered>,
    screen_dimensions: cgmath::Vector2<u32>,
}

impl GUIRenderer {
    pub fn new(device: &wgpu::Device, surface_config: &wgpu::SurfaceConfiguration) -> Self {
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("render_pipeline_layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });

        let gui_shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("gui_shader_module"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../default_shaders/gui_shader.wgsl").into()),
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("gui_pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &gui_shader_module,
                entry_point: "vs_main",
                buffers: &[GUIVertex::format()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &gui_shader_module,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_config.format,
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
        });

        let panel = GUIPanel {
            position: GUIPosition::Relative(cgmath::Vector2::new(
                0.75,
                0.0,
            )),
            dimensions: GUIDimensions::Relative(cgmath::Vector2::new(0.25, 0.25)),
            color: wgpu::Color::RED,
        };

        Self {
            render_pipeline,
            panels: vec![panel.buffer(
                &device,
                cgmath::Vector2::new(surface_config.width, surface_config.height),
            )],
            screen_dimensions: cgmath::Vector2::new(surface_config.width, surface_config.height),
        }
    }

    pub(crate) fn render(
        &self,
        command_encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
    ) {
        let mut gui_render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("gui_render_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        });

        gui_render_pass.set_pipeline(&self.render_pipeline);
        for panel in &self.panels {
            panel.render(&mut gui_render_pass);
        }
    }
}

enum GUIPosition {
    Absolute,
    /// As a percentage of the screen
    Relative(cgmath::Vector2<f32>)
}

enum GUIDimensions {
    Absolute,
    /// As a percentage of the screen
    Relative(cgmath::Vector2<f32>)
}

struct GUIPanel {
    /// Position of the top-left corner of the panel
    position: GUIPosition,
    dimensions: GUIDimensions,
    color: wgpu::Color,
}

impl GUIPanel {
    fn buffer(
        &self,
        device: &wgpu::Device,
        _screen_dimensions: cgmath::Vector2<u32>,
    ) -> GUIPanelBuffered {
        let mut left = 0.0;
        let mut top = 0.0;
        match self.position {
            GUIPosition::Absolute => {}
            GUIPosition::Relative(percentage) => {
                left = -1.0 + (1.0 - (-1.0)) * percentage.x;
                top = -(-1.0 + (1.0 - (-1.0)) * percentage.y);
            }
        }

        let mut right = 0.0;
        let mut bottom = 0.0;
        match self.dimensions {
            GUIDimensions::Absolute => {}
            GUIDimensions::Relative(percentage) => {
                right = -(-1.0 + (1.0 - (-1.0)) * percentage.x);
                bottom = -(-1.0 + (1.0 - (-1.0)) * percentage.y);
            }
        }

        right += left;
        bottom = top - bottom;

        let vertices = vec![
            // Top left
            GUIVertex {
                position: [left, top],
                color: [
                    self.color.r as f32,
                    self.color.g as f32,
                    self.color.b as f32,
                    self.color.a as f32,
                ],
            },
            // Bottom left
            GUIVertex {
                position: [left, bottom],
                color: [
                    self.color.r as f32,
                    self.color.g as f32,
                    self.color.b as f32,
                    self.color.a as f32,
                ],
            },
            // Bottom right
            GUIVertex {
                position: [right, bottom],
                color: [
                    self.color.r as f32,
                    self.color.g as f32,
                    self.color.b as f32,
                    self.color.a as f32,
                ],
            },
            // Top right
            GUIVertex {
                position: [right, top],
                color: [
                    self.color.r as f32,
                    self.color.g as f32,
                    self.color.b as f32,
                    self.color.a as f32,
                ],
            },
        ];

        let indices: Vec<u32> = vec![0, 1, 2, 0, 2, 3];

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("gui_vertex_buffer"),
            contents: &bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("gui_index_buffer"),
            contents: &bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        GUIPanelBuffered {
            vertex_buffer,
            index_buffer,
            indices_len: indices.len() as u32,
        }
    }
}

struct GUIPanelBuffered {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    indices_len: u32,
}

impl GUIPanelBuffered {
    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        render_pass.draw_indexed(0..self.indices_len, 0, 0..1);
    }
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Copy, Clone, Debug)]
struct GUIVertex {
    position: [f32; 2],
    color: [f32; 4],
}

impl GUIVertex {
    fn format<'a>() -> wgpu::VertexBufferLayout<'a> {
        const ATTRIBUTES: [wgpu::VertexAttribute; 2] =
            wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x4];
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &ATTRIBUTES,
        }
    }
}
