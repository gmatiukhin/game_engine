use crate::gfx::gfx_2d::components_2d::Surface2D;
use crate::gfx::gfx_2d::components_2d::*;
use crate::gfx::gfx_2d::text::*;
use crate::util::OPENGL_TO_WGPU_MATRIX;
use log::info;
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

    projection: cgmath::Matrix4<f32>,
    projection_buffer: wgpu::Buffer,
    projection_bind_group: wgpu::BindGroup,

    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    texture_bind_group_layout: wgpu::BindGroupLayout,

    background_surface: Surface2D,
    background_texture_bind_group: wgpu::BindGroup,

    foreground_surface: Surface2D,
    foreground_texture_bind_group: wgpu::BindGroup,

    text_rasterizer: TextRasterizer,
}

impl Renderer2D {
    pub(crate) fn new(
        device: Rc<wgpu::Device>,
        queue: Rc<wgpu::Queue>,
        surface_config: &wgpu::SurfaceConfiguration,
    ) -> Self {
        info!("Creating RendererGUI");
        let screen_size: PhysicalSize<u32> = (surface_config.width, surface_config.height).into();

        let projection = OPENGL_TO_WGPU_MATRIX
            * cgmath::ortho(
                0.0,
                screen_size.width as f32,
                screen_size.height as f32,
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

        let vertices = vec![
            // Top left
            GUIVertex {
                position: [0.0, 0.0],
                text_coords: [0.0, 0.0],
            },
            // Bottom left
            GUIVertex {
                position: [0.0, screen_size.height as f32],
                text_coords: [0.0, 1.0],
            },
            // Bottom right
            GUIVertex {
                position: [screen_size.width as f32, screen_size.height as f32],
                text_coords: [1.0, 1.0],
            },
            // Top right
            GUIVertex {
                position: [screen_size.width as f32, 0.0],
                text_coords: [1.0, 0.0],
            },
        ];

        let indices: Vec<u32> = vec![0, 1, 2, 0, 2, 3];

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("surface_vertex_buffer"),
            contents: &bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("surface_index_buffer"),
            contents: &bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let background_surface = Surface2D::from_color(crate::gfx::texture::Color::BLACK);

        let background_texture = crate::gfx::texture::Texture::from_image(
            &device,
            &queue,
            &background_surface.image(),
            "Background surface texture",
            true,
        );

        let background_texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Background surface texture bind group"),
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&background_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&background_texture.sampler),
                },
            ],
        });

        let foreground_surface = Surface2D::from_color(crate::gfx::texture::Color::WHITE);

        let foreground_texture = crate::gfx::texture::Texture::from_image(
            &device,
            &queue,
            &foreground_surface.image(),
            "Foreground surface texture",
            true,
        );

        let foreground_texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Foreground surface texture bind group"),
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&foreground_texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&foreground_texture.sampler),
                },
            ],
        });

        Self {
            device,
            queue,
            screen_size,
            render_pipeline,
            projection,
            projection_buffer,
            projection_bind_group,
            vertex_buffer,
            index_buffer,
            texture_bind_group_layout,
            background_surface,
            background_texture_bind_group,
            foreground_surface,
            foreground_texture_bind_group,
            text_rasterizer,
        }
    }

    pub(crate) fn render_background(
        &self,
        command_encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
    ) {
        let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("background_render_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: true,
                },
            })],
            depth_stencil_attachment: None,
        });

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.projection_bind_group, &[]);
        render_pass.set_bind_group(1, &self.background_texture_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        render_pass.draw_indexed(0..6, 0, 0..1);
    }

    pub(crate) fn render_foreground(
        &self,
        command_encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
    ) {
        let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("foreground_render_pass"),
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

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.projection_bind_group, &[]);
        render_pass.set_bind_group(1, &self.foreground_texture_bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

        render_pass.draw_indexed(0..6, 0, 0..1);
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

        let background_texture = crate::gfx::texture::Texture::from_image(
            &self.device,
            &self.queue,
            &self.background_surface.image(),
            "Background surface texture",
            true,
        );

        let background_texture_bind_group =
            self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Background surface texture bind group"),
                layout: &self.texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&background_texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&background_texture.sampler),
                    },
                ],
            });

        let foreground_texture = crate::gfx::texture::Texture::from_image(
            &self.device,
            &self.queue,
            &self.foreground_surface.image(),
            "Foreground surface texture",
            true,
        );

        let foreground_texture_bind_group =
            self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Foreground surface texture bind group"),
                layout: &self.texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&foreground_texture.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&foreground_texture.sampler),
                    },
                ],
            });

        self.background_texture_bind_group = background_texture_bind_group;
        self.foreground_texture_bind_group = foreground_texture_bind_group;
    }
}

impl Renderer2D {
    pub fn set_background_surface(&mut self, surface: Surface2D) {
        self.background_surface = surface;
    }

    pub fn background_surface(&mut self) -> &mut Surface2D {
        &mut self.background_surface
    }

    pub fn set_foreground_surface(&mut self, surface: Surface2D) {
        self.foreground_surface = surface;
    }

    pub fn foreground_surface(&mut self) -> &mut Surface2D {
        &mut self.foreground_surface
    }
}
