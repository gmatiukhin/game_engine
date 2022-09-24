use crate::gfx::gfx_3d::camera::{Camera, CameraState};
use crate::gfx::gfx_3d::components_3d::*;
use crate::gfx::texture;
use crate::{ResizeMode, WindowSettings};
use log::info;
use std::collections::HashMap;
use std::rc::Rc;
use winit::dpi::PhysicalSize;

pub mod camera;
pub mod components_3d;

pub struct Renderer3D {
    device: Rc<wgpu::Device>,
    queue: Rc<wgpu::Queue>,
    screen_size: PhysicalSize<u32>,
    surface_format: wgpu::TextureFormat,
    window_settings: WindowSettings,

    depth_texture: texture::Texture,

    camera_state: CameraState,
    texture_bind_group_layout: wgpu::BindGroupLayout,

    models: HashMap<String, (bool, Model)>,
    buffered_models: HashMap<String, (wgpu::RenderPipeline, ModelBuffered)>,

    prefabs: HashMap<String, (wgpu::RenderPipeline, Prefab)>,
}

impl Renderer3D {
    pub(crate) fn new(
        device: Rc<wgpu::Device>,
        queue: Rc<wgpu::Queue>,
        surface_config: &wgpu::SurfaceConfiguration,
        window_settings: WindowSettings,
    ) -> Self {
        info!("Creating Renderer3D");
        let screen_size: PhysicalSize<u32> = (surface_config.width, surface_config.height).into();
        let camera_state = CameraState::default_state(&device, &surface_config);

        let depth_texture = texture::Texture::depth_texture(&device, &surface_config);

        let texture_bind_group_layout =
            device.create_bind_group_layout(&texture::TEXTURE_BIND_GROUP_LAYOUT_DESCRIPTOR);

        Self {
            device,
            queue,
            screen_size,
            surface_format: surface_config.format,
            window_settings,
            depth_texture,
            camera_state,
            texture_bind_group_layout,
            models: HashMap::new(),
            buffered_models: HashMap::new(),
            prefabs: HashMap::new(),
        }
    }

    fn default_vertex_shader_module(&self) -> wgpu::ShaderModule {
        self.device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("mesh_vertex_shader"),
                source: wgpu::ShaderSource::Wgsl(
                    include_str!("../../res/shaders/vertex_default.wgsl").into(),
                ),
            })
    }

    fn instance_vertex_shader_module(&self) -> wgpu::ShaderModule {
        self.device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("instanced_vertex_shader"),
                source: wgpu::ShaderSource::Wgsl(
                    include_str!("../../res/shaders/vertex_instanced.wgsl").into(),
                ),
            })
    }

    fn default_fragment_shader_module(&self) -> wgpu::ShaderModule {
        self.device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("default_fragment_shader"),
                source: wgpu::ShaderSource::Wgsl(
                    include_str!("../../res/shaders/fragment_default.wgsl").into(),
                ),
            })
    }

    fn create_pipeline(
        &self,
        buffer_layouts: &[wgpu::VertexBufferLayout],
        vertex_shader_module: &wgpu::ShaderModule,
        fragment_shader_module: &wgpu::ShaderModule,
        label: &str,
    ) -> wgpu::RenderPipeline {
        let render_pipeline_layout =
            self.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("render_pipeline_layout"),
                    bind_group_layouts: &[
                        &self.camera_state.camera_bind_group_layout,
                        &self.texture_bind_group_layout,
                    ],
                    push_constant_ranges: &[],
                });

        self.device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &vertex_shader_module,
                    entry_point: "vs_main",
                    buffers: buffer_layouts,
                },
                fragment: Some(wgpu::FragmentState {
                    module: &fragment_shader_module,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: self.surface_format,
                        // In order to have transparency you should implement Order Independent Transparency algorithm
                        // or sort all of the objects
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::all(),
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    unclipped_depth: false,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: texture::DEPTH_TEXTURE_FORMAT,

                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
            })
    }

    pub(crate) fn render_scene(
        &self,
        command_encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
    ) {
        let mut render_pass = command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("render_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: true,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_texture.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: true,
                }),
                stencil_ops: None,
            }),
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
            render_pass.set_scissor_rect(
                scissors_x as u32,
                scissors_y as u32,
                scissors_width as u32,
                scissors_height as u32,
            );
        }

        render_pass.set_bind_group(0, &self.camera_state.camera_bind_group, &[]);

        for (_, (pipeline, model)) in &self.buffered_models {
            render_pass.set_pipeline(pipeline);
            model.render(&mut render_pass, 0..1);
        }

        for (_, (pipeline, prefab)) in &self.prefabs {
            render_pass.set_pipeline(pipeline);
            prefab.render(&mut render_pass);
        }
    }

    pub(crate) fn resize(
        &mut self,
        new_size: PhysicalSize<u32>,
        surface_config: &wgpu::SurfaceConfiguration,
    ) {
        self.screen_size = new_size;
        self.depth_texture = texture::Texture::depth_texture(&self.device, &surface_config);
        self.camera_state
            .camera
            .resize(new_size.width, new_size.height);
    }

    pub(crate) fn update(&mut self) {
        self.camera_state.update(&self.queue);
        self.buffer_models();
    }

    pub fn camera(&mut self) -> &mut Camera {
        &mut self.camera_state.camera
    }
}

/// Methods related to models
impl Renderer3D {
    pub fn add_model(&mut self, model: Model) {
        self.models.insert(model.name.clone(), (true, model));
    }

    pub fn get_model(&mut self, name: &str) -> Option<&mut Model> {
        self.models.get_mut(name).map(|(_, m)| m)
    }

    pub fn remove_model(&mut self, name: &str) {
        self.models.remove(name);
        self.buffered_models.remove(name);
    }

    fn buffer_models(&mut self) {
        for (name, (should_buffer, model)) in &self.models {
            if *should_buffer {
                let buff_model =
                    model.buffer(&self.device, &self.queue, &self.texture_bind_group_layout);
                let render_pipeline = self.create_pipeline(
                    &[VertexRaw::format()],
                    &self.default_vertex_shader_module(),
                    &buff_model
                        .shader_module
                        .as_ref()
                        .unwrap_or(&self.default_fragment_shader_module()),
                    &format!("Render pipeline for model {}", buff_model.name),
                );

                self.buffered_models
                    .insert(name.clone(), (render_pipeline, buff_model));
            }
        }
    }
}

/// Methods related to prefabs
impl Renderer3D {
    pub fn add_as_prefab(&mut self, model: &Model) -> String {
        let model = model.buffer(&self.device, &self.queue, &self.texture_bind_group_layout);

        let render_pipeline = self.create_pipeline(
            &[VertexRaw::format(), InstanceTransformRaw::format()],
            &self.instance_vertex_shader_module(),
            &model
                .shader_module
                .as_ref()
                .unwrap_or(&self.default_fragment_shader_module()),
            &format!("Render pipeline for model {}", model.name),
        );

        let prefab = Prefab {
            name: model.name.clone(),
            model,
            transforms: HashMap::new(),
            instance_buffer: None,
        };

        let name = prefab.name.clone();

        self.prefabs
            .insert(prefab.name.clone(), (render_pipeline, prefab));

        name
    }

    pub fn instantiate_prefab(
        &mut self,
        prefab_name: &str,
        position: &cgmath::Point3<f32>,
        rotation: &cgmath::Quaternion<f32>,
    ) -> Option<PrefabInstance> {
        let mut instance_handle = None;
        self.prefabs
            .entry(prefab_name.to_string())
            .and_modify(|(_, prefab)| {
                instance_handle = Some(prefab.add_instance(position, rotation));
                prefab.update_buffer(&self.device);
            });

        instance_handle
    }

    pub fn update_prefab_instance(&mut self, instance: &PrefabInstance) {
        self.prefabs
            .entry(instance.name.clone())
            .and_modify(|(_, prefab)| {
                prefab.update_instance(instance);
                prefab.update_buffer(&self.device);
            });
    }

    pub fn delete_prefab_instance(&mut self, instance: &PrefabInstance) {
        self.prefabs
            .entry(instance.name.clone())
            .and_modify(|(_, prefab)| {
                prefab.remove_instance(instance);
                prefab.update_buffer(&self.device);
            });
    }
}
