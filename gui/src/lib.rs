use wgpu::util::DeviceExt;

pub struct GUIRenderer {
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    indices: Vec<u32>,
}

impl GUIRenderer {
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat) -> Self {
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("render_pipeline_layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });

        let gui_shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("gui_shader_module"),
            source: wgpu::ShaderSource::Wgsl(include_str!("gui_shader.wgsl").into()),
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
                    format: surface_format,
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

        let vertices = vec![
            GUIVertex {
                position: [0.5, 0.5],
                debug_color: [1.0, 0.0, 0.0, 1.0],
            },
            GUIVertex {
                position: [1.0, 0.5],
                debug_color: [1.0, 0.0, 0.0, 1.0],
            },
            GUIVertex {
                position: [1.0, 1.0],
                debug_color: [1.0, 0.0, 0.0, 1.0],
            },
            GUIVertex {
                position: [0.5, 1.0],
                debug_color: [1.0, 0.0, 0.0, 1.0],
            },
        ];

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("gui_vertex_buffer"),
            contents: &bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let indices: Vec<u32> = vec![0, 1, 3, 3, 1, 2];

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("gui_index_buffer"),
            contents: &bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        Self {
            render_pipeline,
            vertex_buffer,
            index_buffer,
            indices,
        }
    }

    pub fn render<'a>(&self, command_encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
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
        gui_render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        gui_render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        gui_render_pass.draw_indexed(0..self.indices.len() as u32, 0, 0..1);
    }
}

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Copy, Clone)]
struct GUIVertex {
    position: [f32; 2],
    debug_color: [f32; 4],
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
