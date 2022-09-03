use crate::gfx::gfx_2d::components_2d::*;
use crate::gfx::gfx_2d::text::*;
use crate::util::OPENGL_TO_WGPU_MATRIX;
use log::info;
use std::collections::vec_deque::VecDeque;
use std::rc::Rc;
use wgpu::util::DeviceExt;
use winit::dpi::PhysicalSize;

pub mod components_2d;
pub mod text;

pub struct Renderer2D {
    device: Rc<wgpu::Device>,
    queue: Rc<wgpu::Queue>,

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

impl Renderer2D {
    pub fn add_top_level_panels(&mut self, mut panels: Vec<GUIPanel>) {
        self.panels.append(&mut panels);
    }

    pub fn get_panel(&mut self, name: &str) -> Option<&mut GUIPanel> {
        let mut panel_queue = VecDeque::new();
        for panel in &mut self.panels {
            panel_queue.push_back(panel);
        }

        loop {
            if panel_queue.is_empty() {
                break;
            }
            if let Some(panel) = panel_queue.pop_front() {
                if panel.name == name {
                    return Some(panel);
                } else {
                    if let GUIPanelContent::Panels(_, children) = &mut panel.content {
                        for child in children {
                            panel_queue.push_back(child);
                        }
                    }
                }
            }
        }

        None
    }
}

impl Renderer2D {
    pub(crate) fn new(
        device: Rc<wgpu::Device>,
        queue: Rc<wgpu::Queue>,
        surface_config: &wgpu::SurfaceConfiguration,
    ) -> Self {
        info!("Creating RendererGUI");
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

        let texture_bind_group_layout = device
            .create_bind_group_layout(&crate::gfx::texture::TEXTURE_BIND_GROUP_LAYOUT_DESCRIPTOR);

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
                    include_str!("../../res/shaders/gui_shader.wgsl").into(),
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

        let text_rasterizer = TextRasterizer::new();

        Self {
            device,
            queue,
            screen_size: (surface_config.width, surface_config.height).into(),
            render_pipeline,
            panels: vec![],
            buffered_panels: vec![],
            projection,
            projection_buffer,
            projection_bind_group,
            texture_bind_group_layout,
            text_rasterizer,
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

    pub(crate) fn update(&mut self) {
        let projection_raw: [[f32; 4]; 4] = self.projection.into();
        self.queue.write_buffer(
            &self.projection_buffer,
            0,
            bytemuck::cast_slice(&[projection_raw]),
        );

        self.buffered_panels = self
            .panels
            .iter_mut()
            .map(|panel| {
                panel.buffer(
                    &self.device,
                    &self.queue,
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