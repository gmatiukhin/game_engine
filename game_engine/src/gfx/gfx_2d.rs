use crate::gfx::gfx_2d::components_2d::Sprite;
use crate::util::OPENGL_TO_WGPU_MATRIX;
use crate::{ResizeMode, WindowSettings};
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
    window_settings: WindowSettings,
    render_pipeline: wgpu::RenderPipeline,

    projection: cgmath::Matrix4<f32>,
    projection_buffer: wgpu::Buffer,
    projection_bind_group: wgpu::BindGroup,

    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    texture_bind_group_layout: wgpu::BindGroupLayout,

    background_sprite: Sprite,
    background_texture_bind_group: wgpu::BindGroup,

    foreground_sprite: Sprite,
    foreground_texture_bind_group: wgpu::BindGroup,
}

impl Renderer2D {
    pub(crate) fn new(
        device: Rc<wgpu::Device>,
        queue: Rc<wgpu::Queue>,
        surface_config: &wgpu::SurfaceConfiguration,
        window_settings: WindowSettings,
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
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2],
                    }],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &gui_shader_module,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: surface_config.format,
                        blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
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

        let vertices = Self::create_screen_size_square(screen_size);

        let indices: Vec<u32> = vec![0, 1, 2, 0, 2, 3];

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("surface_vertex_buffer"),
            contents: &bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("surface_index_buffer"),
            contents: &bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        let background_surface = Sprite::new(
            screen_size.width,
            screen_size.height,
            crate::gfx::texture::PixelColor::BLACK,
        );

        let background_texture = crate::gfx::texture::Texture::from_image(
            &device,
            &queue,
            &background_surface.image(),
            "Background surface texture",
            true,
        );

        let background_texture_view_resource =
            wgpu::BindingResource::TextureView(&background_texture.view);

        let background_texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Background surface texture bind group"),
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: background_texture_view_resource,
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&background_texture.sampler),
                },
            ],
        });

        let foreground_surface = Sprite::new(
            screen_size.width,
            screen_size.height,
            crate::gfx::texture::PixelColor::TRANSPARENT,
        );

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
            window_settings,
            render_pipeline,
            projection,
            projection_buffer,
            projection_bind_group,
            vertex_buffer,
            index_buffer,
            texture_bind_group_layout,
            background_sprite: background_surface,
            background_texture_bind_group,
            foreground_sprite: foreground_surface,
            foreground_texture_bind_group,
        }
    }

    pub(crate) fn render_background(
        &self,
        command_encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
    ) {
        self.render_panel(command_encoder, view, &self.background_texture_bind_group);
    }

    pub(crate) fn render_foreground(
        &self,
        command_encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
    ) {
        self.render_panel(command_encoder, view, &self.foreground_texture_bind_group);
    }

    fn render_panel(
        &self,
        command_encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        panel_bind_group: &wgpu::BindGroup,
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

        if self.window_settings.resize_mode == ResizeMode::KeepAspectRatio {
            let aspect = self.window_settings.window_width as f32
                / self.window_settings.window_height as f32;
            // set up scissors rect with constant aspect ratio that stays in the center
            let (width, height): (f32, f32) = self.screen_size.to_logical::<f32>(1.0).into();
            let (scissors_width, scissors_height) = if width > height * aspect {
                (height * aspect, height)
            } else {
                (width, width / aspect)
            };
            let scissors_x = (width - scissors_width) / 2.0;
            let scissors_y = (height - scissors_height) / 2.0;
            render_pass.set_viewport(
                scissors_x,
                scissors_y,
                scissors_width,
                scissors_height,
                0.0,
                1.0,
            );
        }

        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.set_bind_group(0, &self.projection_bind_group, &[]);
        render_pass.set_bind_group(1, &panel_bind_group, &[]);
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

        let vertices = Self::create_screen_size_square(self.screen_size);

        self.queue
            .write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vertices));

        if self.window_settings.resize_mode != ResizeMode::KeepAspectRatio {
            self.background_sprite.resize(new_size);
            self.foreground_sprite.resize(new_size);
        }
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
            &self.background_sprite.image(),
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
            &self.foreground_sprite.image(),
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

    #[rustfmt::skip]
    fn create_screen_size_square(screen_size: PhysicalSize<u32>) -> [f32; 16] {
        [
            // Top left
            0.0, 0.0,
            0.0, 0.0,
            // Bottom left
            0.0, screen_size.height as f32,
            0.0, 1.0,
            // Bottom right
            screen_size.width as f32, screen_size.height as f32,
            1.0, 1.0,
            // Top right
            screen_size.width as f32, 0.0,
            1.0, 0.0,
        ]
    }
}

impl Renderer2D {
    pub fn background(&mut self) -> &mut Sprite {
        &mut self.background_sprite
    }

    pub fn foreground(&mut self) -> &mut Sprite {
        &mut self.foreground_sprite
    }
}
