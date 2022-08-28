use crate::util::OPENGL_TO_WGPU_MATRIX;
use wgpu::util::DeviceExt;
use winit::dpi::PhysicalSize;

pub struct GUIRenderer {
    screen_size: PhysicalSize<u32>,
    render_pipeline: wgpu::RenderPipeline,
    panels: Vec<GUIPanel>,
    projection: cgmath::Matrix4<f32>,
    projection_buffer: wgpu::Buffer,
    projection_bind_group: wgpu::BindGroup,
}

impl GUIRenderer {
    pub fn new(device: &wgpu::Device, surface_config: &wgpu::SurfaceConfiguration) -> Self {
        let projection = OPENGL_TO_WGPU_MATRIX
            * cgmath::ortho(
                0.0,
                surface_config.width as f32,
                surface_config.height as f32,
                0.0,
                -1.0,
                1000.0,
            );

        let projection_raw: [[f32; 4]; 4] = projection.into();

        let projection_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("projection_buffer"),
            contents: bytemuck::cast_slice(&projection_raw),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let projection_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let projection_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("projection_bind_group"),
            layout: &projection_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: projection_buffer.as_entire_binding(),
            }],
        });

        let render_pipeline = {
            let render_pipeline_layout =
                device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("render_pipeline_layout"),
                    bind_group_layouts: &[&projection_bind_group_layout],
                    push_constant_ranges: &[],
                });

            let gui_shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("gui_shader_module"),
                source: wgpu::ShaderSource::Wgsl(
                    include_str!("../default_shaders/gui_shader.wgsl").into(),
                ),
            });

            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
            })
        };
        
        let panels = {
            let child1 = GUIPanel {
                position: GUITransform::Relative(0.2, 0.1),
                dimensions: GUITransform::Relative(0.6, 0.1),
                color: wgpu::Color::GREEN,
                children: vec![]
            };

            let child21 = GUIPanel {
                position: GUITransform::Relative(0.15, 0.1),
                dimensions: GUITransform::Relative(0.7, 0.35),
                color: wgpu::Color::WHITE,
                children: vec![]
            };

            let child22 = GUIPanel {
                position: GUITransform::Relative(0.1, 0.55),
                dimensions: GUITransform::Relative(0.8, 0.35),
                color: wgpu::Color::WHITE,
                children: vec![]
            };

            let child2 = GUIPanel {
                position: GUITransform::Relative(0.1, 0.3),
                dimensions: GUITransform::Relative(0.8, 0.5),
                color: wgpu::Color::BLUE,
                children: vec![child21, child22]
            };

            let top_panel = GUIPanel {
                position: GUITransform::Absolute(10, 10),
                dimensions: GUITransform::Relative(0.3, 0.8),
                color: wgpu::Color::RED,
                children: vec![child1, child2]
            };
            
            vec![top_panel]
        };

        Self {
            screen_size: (surface_config.width, surface_config.height).into(),
            render_pipeline,
            panels,
            projection,
            projection_buffer,
            projection_bind_group,
        }
    }

    pub(crate) fn render(
        &self,
        command_encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        device: &wgpu::Device,
    ) {
        let buffered_panels = &self
            .panels
            .iter()
            .map(|panel| {
                panel.buffer(
                    device,
                    (0.0, 0.0).into(),
                    (self.screen_size.width as f32, self.screen_size.height as f32).into(),
                )
            })
            .collect::<Vec<GUIPanelBuffered>>();

        {
            let mut gui_render_pass =
                command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
            gui_render_pass.set_bind_group(0, &self.projection_bind_group, &[]);
            for panel in buffered_panels {
                panel.render(&mut gui_render_pass);
            }
        }
    }

    pub(crate) fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.screen_size = new_size;
        self.projection = OPENGL_TO_WGPU_MATRIX
            * cgmath::ortho(
                0.0,
                self.screen_size.width as f32,
                self.screen_size.height as f32,
                0.0,
                -1.0,
                1000.0,
            );
    }

    pub(crate) fn update(&mut self, queue: &wgpu::Queue) {
        let projection_raw: [[f32; 4]; 4] = self.projection.into();
        queue.write_buffer(&self.projection_buffer, 0, bytemuck::cast_slice(&[projection_raw]));
    }
}

enum GUITransform {
    /// Pixel values
    Absolute(u32, u32),
    /// As a percentage of the corresponding parent transform
    Relative(f32, f32),
}

struct GUIPanel {
    /// Position of the top-left corner of the panel
    position: GUITransform,
    dimensions: GUITransform,
    color: wgpu::Color,

    children: Vec<GUIPanel>,
}

impl GUIPanel {
    fn buffer(
        &self,
        device: &wgpu::Device,
        parent_anchor: cgmath::Vector2<f32>,
        parent_dimensions: cgmath::Vector2<f32>,
    ) -> GUIPanelBuffered {
        let (left, top) = match self.position {
            GUITransform::Absolute(x, y) => (
                parent_anchor.x + x as f32,
                parent_anchor.y + y as f32
            ),
            GUITransform::Relative(percentage_x, percentage_y) => (
                parent_anchor.x + (0.0 + parent_dimensions.x as f32 * percentage_x),
                parent_anchor.y + (0.0 + parent_dimensions.y as f32 * percentage_y),
            ),
        };

        let (right, bottom) = match self.dimensions {
            GUITransform::Absolute(width, height) => (left + width as f32, top + height as f32),
            GUITransform::Relative(percentage_x, percentage_y) => (
                left + (0.0 + parent_dimensions.x as f32 * percentage_x),
                top + (0.0 + parent_dimensions.y as f32 * percentage_y),
            ),
        };

        let color_as_array = |color: &wgpu::Color| -> [f32; 4] {
            [
                color.r as f32,
                color.g as f32,
                color.b as f32,
                color.a as f32,
            ]
        };

        let vertices = vec![
            // Top left
            GUIVertex {
                position: [left, top],
                color: color_as_array(&self.color),
            },
            // Bottom left
            GUIVertex {
                position: [left, bottom],
                color: color_as_array(&self.color),
            },
            // Bottom right
            GUIVertex {
                position: [right, bottom],
                color: color_as_array(&self.color),
            },
            // Top right
            GUIVertex {
                position: [right, top],
                color: color_as_array(&self.color),
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

        let buffered_children = self.children.iter().map(|child| child.buffer(device, (left, top).into(), (right - left, bottom - top).into())).collect::<Vec<_>>();

        GUIPanelBuffered {
            vertex_buffer,
            index_buffer,
            indices_len: indices.len() as u32,
            children: buffered_children
        }
    }
}

struct GUIPanelBuffered {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    indices_len: u32,

    children: Vec<GUIPanelBuffered>,
}

impl GUIPanelBuffered {
    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..self.indices_len, 0, 0..1);

        for child in &self.children {
            child.render(render_pass);
        }
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
