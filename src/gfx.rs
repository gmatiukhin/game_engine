use log::info;
use std::collections::HashMap;
use std::rc::Rc;
use winit::dpi::PhysicalSize;

use crate::gfx::camera::{Camera, CameraState};
pub use wgpu::Color;

pub mod camera;
pub mod components;
use components::*;

pub mod material;

pub struct Renderer3D {
    device: Rc<wgpu::Device>,
    queue: Rc<wgpu::Queue>,
    // surface_config: wgpu::SurfaceConfiguration,
    surface_format: wgpu::TextureFormat,

    depth_texture: material::Texture,

    camera_state: CameraState,
    texture_bind_group_layout: wgpu::BindGroupLayout,

    models: HashMap<String, (wgpu::RenderPipeline, ModelBuffered)>,
    prefabs: HashMap<String, (wgpu::RenderPipeline, Prefab)>,
}

impl Renderer3D {
    pub(crate) fn new(
        device: Rc<wgpu::Device>,
        queue: Rc<wgpu::Queue>,
        surface_config: &wgpu::SurfaceConfiguration,
    ) -> Self {
        info!("Creating Renderer3D");

        let camera_state = CameraState::default_state(&device, &surface_config);

        let depth_texture = material::Texture::depth_texture(&device, &surface_config);

        let texture_bind_group_layout = device
            .create_bind_group_layout(&material::Texture::TEXTURE_BIND_GROUP_LAYOUT_DESCRIPTOR);

        Self {
            device,
            queue,
            surface_format: surface_config.format,
            depth_texture,
            camera_state,
            texture_bind_group_layout,
            models: HashMap::new(),
            prefabs: HashMap::new(),
        }
    }

    fn default_vertex_shader_module(&self) -> wgpu::ShaderModule {
        self.device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("mesh_vertex_shader"),
                source: wgpu::ShaderSource::Wgsl(
                    include_str!("default_shaders/vertex_default.wgsl").into(),
                ),
            })
    }

    fn instance_vertex_shader_module(&self) -> wgpu::ShaderModule {
        self.device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("instanced_vertex_shader"),
                source: wgpu::ShaderSource::Wgsl(
                    include_str!("default_shaders/vertex_instanced.wgsl").into(),
                ),
            })
    }

    fn default_fragment_shader_module(&self) -> wgpu::ShaderModule {
        self.device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("default_fragment_shader"),
                source: wgpu::ShaderSource::Wgsl(
                    include_str!("default_shaders/fragment_default.wgsl").into(),
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
                    cull_mode: Some(wgpu::Face::Back),
                    unclipped_depth: false,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: material::Texture::DEPTH_TEXTURE_FORMAT,
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

    pub(crate) fn render(
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
                    load: wgpu::LoadOp::Clear(Color {
                        r: 0.1,
                        g: 0.2,
                        b: 0.3,
                        a: 1.0,
                    }),
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
        render_pass.set_bind_group(0, &self.camera_state.camera_bind_group, &[]);

        for (_, (pipeline, model)) in &self.models {
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
        self.depth_texture = material::Texture::depth_texture(&self.device, &surface_config);
        self.camera_state
            .camera
            .resize(new_size.width, new_size.height);
    }

    pub(crate) fn update(&mut self) {
        self.camera_state.update(&self.queue);
    }

    pub fn camera(&mut self) -> &mut Camera {
        &mut self.camera_state.camera
    }
}

impl Renderer3D {
    pub fn add_model(&mut self, model: &Model) {
        let model = model.buffer(&self.device, &self.queue, &self.texture_bind_group_layout);

        let render_pipeline = self.create_pipeline(
            &[VertexRaw::format()],
            &self.default_vertex_shader_module(),
            &model
                .shader_module
                .as_ref()
                .unwrap_or(&self.default_fragment_shader_module()),
            &format!("Render pipeline for model {}", model.name),
        );

        self.models
            .insert(model.name.clone(), (render_pipeline, model));
    }

    pub fn update_model(&mut self, model: &Model) {
        self.models.entry(model.name.clone()).and_modify(|(_, m)| {
            *m = model.buffer(&self.device, &self.queue, &self.texture_bind_group_layout)
        });
    }

    pub fn delete_model(&mut self, model: Model) {
        self.models.remove(&model.name);
    }
}

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
