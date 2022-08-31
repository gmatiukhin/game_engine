use crate::util::OPENGL_TO_WGPU_MATRIX;
use wgpu::util::DeviceExt;
use winit::dpi::PhysicalSize;
use crate::text::{TextRasterizer, TextParameters};

pub struct GUIRenderer {
    screen_size: PhysicalSize<u32>,
    render_pipeline: wgpu::RenderPipeline,

    panels: Vec<GUIPanel>,
    buffered_panels: Vec<GUIPanelBuffered>,

    projection: cgmath::Matrix4<f32>,
    projection_buffer: wgpu::Buffer,
    projection_bind_group: wgpu::BindGroup,
    texture_bind_group_layout: wgpu::BindGroupLayout,

    text_rasterizer: TextRasterizer,
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

        let projection_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

        let texture_bind_group_layout = device.create_bind_group_layout(
            &crate::gfx::material::Texture::TEXTURE_BIND_GROUP_LAYOUT_DESCRIPTOR,
        );

        let render_pipeline = {
            let render_pipeline_layout =
                device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("render_pipeline_layout"),
                    bind_group_layouts: &[
                        &projection_bind_group_layout,
                        &texture_bind_group_layout,
                    ],
                    push_constant_ranges: &[],
                });

            let gui_shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("gui_shader_module"),
                source: wgpu::ShaderSource::Wgsl(
                    include_str!("default_shaders/gui_shader.wgsl").into(),
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
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
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

        let buffer = std::fs::read("./res/textures/stone_bricks.jpg").unwrap();
        let image = image::load_from_memory(&buffer).unwrap();

        let panel_texture = GUIPanel {
            name: "Test texture".to_string(),
            active: false,
            position: GUITransform::Relative(0.1, 0.1),
            dimensions: GUITransform::Relative(0.8, 0.3),
            content: GUIPanelContent::Image(crate::gfx::material::Image {
                name: "stone_brick".to_string(),
                file: image,
            }),
        };

        let panel_text = GUIPanel {
            name: "Test text".to_string(),
            active: true,
            position: GUITransform::Relative(0.1, 0.5),
            dimensions: GUITransform::Relative(0.8, 0.4),
            content: GUIPanelContent::Text(TextParameters {
                text: "hello world, hello world, hello world".to_string(),
                color: wgpu::Color::GREEN,
                scale: 20.0
            }),
        };

        let panel_color = GUIPanel {
            name: "Test color".to_string(),
            active: true,
            position: GUITransform::Relative(0.01, 0.01),
            dimensions: GUITransform::Relative(0.3, 0.7),
            content: GUIPanelContent::Elements(wgpu::Color::BLACK, vec![panel_texture, panel_text]),
        };

        let text_rasterizer = TextRasterizer::new();

        Self {
            screen_size: (surface_config.width, surface_config.height).into(),
            render_pipeline,
            panels: vec![panel_color],
            buffered_panels: vec![],
            projection,
            projection_buffer,
            projection_bind_group,
            texture_bind_group_layout,
            text_rasterizer
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
        gui_render_pass.set_bind_group(0, &self.projection_bind_group, &[]);
        for panel in &self.buffered_panels {
            panel.render(&mut gui_render_pass);
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

    pub(crate) fn update(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let projection_raw: [[f32; 4]; 4] = self.projection.into();
        queue.write_buffer(
            &self.projection_buffer,
            0,
            bytemuck::cast_slice(&[projection_raw]),
        );

        self.buffered_panels = self
            .panels
            .iter()
            .map(|panel| {
                panel.buffer(
                    device,
                    queue,
                    &self.texture_bind_group_layout,
                    &self.text_rasterizer,
                    (0.0, 0.0).into(),
                    (
                        self.screen_size.width as f32,
                        self.screen_size.height as f32,
                    )
                        .into(),
                )
            })
            .filter(|el| if let Some(_) = el { true } else { false })
            .map(|el| el.unwrap())
            .collect();
    }
}

enum GUITransform {
    /// Pixel values
    Absolute(u32, u32),
    /// As a percentage of the corresponding parent transform
    Relative(f32, f32),
}

enum GUIPanelContent {
    Image(crate::gfx::material::Image),
    Text(crate::text::TextParameters),
    Elements(wgpu::Color, Vec<GUIPanel>),
}

struct GUIPanel {
    name: String,
    active: bool,
    /// Position of the top-left corner of the panel
    position: GUITransform,
    dimensions: GUITransform,

    content: GUIPanelContent,
}

impl GUIPanel {
    fn buffer(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        text_rasterizer: &TextRasterizer,
        parent_anchor: cgmath::Vector2<f32>,
        parent_dimensions: cgmath::Vector2<f32>,
    ) -> Option<GUIPanelBuffered> {
        if !self.active {
            return None;
        }

        let (left, top) = match self.position {
            GUITransform::Absolute(x, y) => {
                (parent_anchor.x + x as f32, parent_anchor.y + y as f32)
            }
            GUITransform::Relative(percentage_x, percentage_y) => (
                parent_anchor.x + parent_dimensions.x as f32 * percentage_x,
                parent_anchor.y + parent_dimensions.y as f32 * percentage_y,
            ),
        };

        let (right, bottom) = match self.dimensions {
            GUITransform::Absolute(width, height) => (left + width as f32, top + height as f32),
            GUITransform::Relative(percentage_x, percentage_y) => (
                left + parent_dimensions.x as f32 * percentage_x,
                top + parent_dimensions.y as f32 * percentage_y,
            ),
        };

        let left = left
            .max(parent_anchor.x)
            .min(parent_dimensions.x + parent_anchor.x);
        let top = top
            .max(parent_anchor.y)
            .min(parent_dimensions.y + parent_anchor.y);
        let right = right
            .max(parent_anchor.x)
            .min(parent_dimensions.x + parent_anchor.x);
        let bottom = bottom
            .max(parent_anchor.y)
            .min(parent_dimensions.y + parent_anchor.y);

        let vertices = vec![
            // Top left
            GUIVertex {
                position: [left, top],
                text_coords: [0.0, 0.0],
            },
            // Bottom left
            GUIVertex {
                position: [left, bottom],
                text_coords: [0.0, 1.0],
            },
            // Bottom right
            GUIVertex {
                position: [right, bottom],
                text_coords: [1.0, 1.0],
            },
            // Top right
            GUIVertex {
                position: [right, top],
                text_coords: [1.0, 0.0],
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

        let (texture, children) = match &self.content {
            GUIPanelContent::Image(img) => (
                crate::gfx::material::Texture::from_image(device, queue, &img.file, &img.name),
                vec![],
            ),
            GUIPanelContent::Text(text) => {
                let width: u32 = (right - left) as u32;
                let height: u32 = (bottom - top) as u32;
                let data = text_rasterizer.get_rasterized_data_from_text(text, width, height);
                (
                    crate::gfx::material::Texture::from_text(device, queue, data, width, height),
                    vec![],
                )
            },
            GUIPanelContent::Elements(color, children) => {
                let mut buffered_children: Vec<GUIPanelBuffered> = vec![];
                for child in children {
                    if let Some(panel_buffered) = child.buffer(
                        &device,
                        &queue,
                        &texture_bind_group_layout,
                        text_rasterizer,
                        (left, top).into(),
                        (right - left, bottom - top).into(),
                    ) {
                        buffered_children.push(panel_buffered);
                    }
                }

                (
                    crate::gfx::material::Texture::from_color(device, queue, color),
                    buffered_children,
                )
            }
        };

        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("panel"),
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&texture.sampler),
                },
            ],
        });

        Some(GUIPanelBuffered {
            vertex_buffer,
            index_buffer,
            indices_len: indices.len() as u32,
            texture_bind_group,
            children,
        })
    }
}

#[derive(Debug)]
struct GUIPanelBuffered {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    indices_len: u32,
    texture_bind_group: wgpu::BindGroup,
    children: Vec<GUIPanelBuffered>,
}

impl GUIPanelBuffered {
    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        render_pass.set_bind_group(1, &self.texture_bind_group, &[]);
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
    /// In wgpu's coordinate system UV origin is situated in the top left corner
    text_coords: [f32; 2],
}

impl GUIVertex {
    fn format<'a>() -> wgpu::VertexBufferLayout<'a> {
        const ATTRIBUTES: [wgpu::VertexAttribute; 2] =
            wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2];
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &ATTRIBUTES,
        }
    }
}