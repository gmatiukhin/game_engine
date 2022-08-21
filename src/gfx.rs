use std::collections::hash_map::DefaultHasher;
use log::info;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use winit::dpi::PhysicalSize;
use winit::window::Window;

use crate::gfx::camera::{Camera, CameraState};
pub use wgpu::Color;
use wgpu::ShaderModule;

pub mod camera;
pub mod components;
use components::*;

pub struct Renderer {
    screen_size: PhysicalSize<u32>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface,
    config: wgpu::SurfaceConfiguration,
    mesh_render_pipeline: Option<wgpu::RenderPipeline>,
    instance_render_pipeline: Option<wgpu::RenderPipeline>,

    camera_state: CameraState,

    meshes: HashMap<String, MeshRaw>,
    prefabs: HashMap<String, Prefab>,

    hasher: Box<dyn Hasher>,
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

        let screen_size = window.inner_size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_supported_formats(&adapter)[0],
            width: screen_size.width,
            height: screen_size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };

        surface.configure(&device, &config);

        let camera_state = CameraState::default_state(&device, &config);

        Self {
            screen_size,
            device,
            queue,
            surface,
            config,
            mesh_render_pipeline: None,
            instance_render_pipeline: None,
            camera_state,
            meshes: HashMap::new(),
            prefabs: HashMap::new(),
            hasher: Box::new(DefaultHasher::new()),
        }
    }

    pub(crate) fn init_pipelines(&mut self) {
        let vertex_shader_module = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("mesh_vertex_shader"),
                source: wgpu::ShaderSource::Wgsl(
                    include_str!("../res/vertex_default.wgsl").into(),
                ),
            });

        let fragment_shader_module =
            self.device
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("default_fragment_shader"),
                    source: wgpu::ShaderSource::Wgsl(
                        include_str!("../res/fragment_default.wgsl").into(),
                    ),
                });

        self.mesh_render_pipeline = Some(self.create_pipeline(
            "mesh_render_pipeline",
            &[VertexRaw::format()],
            &vertex_shader_module,
            &fragment_shader_module,
        ));

        if !self.prefabs.is_empty() {
            let vertex_shader_module =
                self.device
                    .create_shader_module(wgpu::ShaderModuleDescriptor {
                        label: Some("instance_vertex_shader"),
                        source: wgpu::ShaderSource::Wgsl(
                            include_str!("../res/vertex_instanced.wgsl").into(),
                        ),
                    });

            self.instance_render_pipeline = Some(self.create_pipeline(
                "instance_render_pipeline",
                &[VertexRaw::format(), InstanceTransformRaw::format()],
                &vertex_shader_module,
                &fragment_shader_module,
            ));
        }
    }

    fn create_pipeline(
        &mut self,
        label: &str,
        buffer_layouts: &[wgpu::VertexBufferLayout],
        vertex_shader_module: &ShaderModule,
        fragment_shader_module: &ShaderModule,
    ) -> wgpu::RenderPipeline {
        let render_pipeline_layout =
            self.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("render_pipeline_layout"),
                    bind_group_layouts: &[&self.camera_state.camera_bind_group_layout],
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
            })
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
            render_pass.set_bind_group(0, &self.camera_state.camera_bind_group, &[]);
            if let Some(pipeline) = &self.mesh_render_pipeline {
                render_pass.set_pipeline(pipeline);

                for (_, mesh) in &self.meshes {
                    mesh.render(&mut render_pass);
                }
            }

            if let Some(pipeline) = &self.instance_render_pipeline {
                render_pass.set_pipeline(pipeline);

                for (_, prefab) in &self.prefabs {
                    prefab.render(&mut render_pass);
                }
            }
        }

        self.queue.submit(std::iter::once(command_encoder.finish()));
        surface_texture.present();

        Ok(())
    }

    pub(crate) fn reload_view(&mut self) {
        self.resize(self.screen_size);
    }

    pub(crate) fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.screen_size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            self.camera_state
                .camera
                .resize(new_size.width, new_size.height);
        }
    }

    pub(crate) fn update_components(&mut self) {
        self.camera_state.update(&self.queue);
        self.update_instances();
    }

    fn update_instances(&mut self) {
        for (_, prefab) in &mut self.prefabs {
            prefab.update_buffer(&self.device);
        }
    }
}

impl Renderer {
    pub fn add_mesh(&mut self, mesh: &Mesh) {
        self.meshes
            .insert(mesh.name.clone(), mesh.as_raw(&self.device));
    }

    pub fn update_mesh(&mut self, mesh: &Mesh) {
        if let Some(mesh_raw) = self.meshes.get_mut(&mesh.name) {
            *mesh_raw = mesh.as_raw(&self.device);
        }
    }

    pub fn add_as_prefab(&mut self, mesh: &Mesh) {
        self.prefabs.insert(
            mesh.name.clone(),
            Prefab {
                name: mesh.name.clone(),
                mesh: mesh.as_raw(&self.device),
                transforms: HashMap::new(),
                instance_buffer: None,
            },
        );
    }

    pub fn instantiate_prefab(
        &mut self,
        prefab_name: &str,
        position: cgmath::Point3<f32>,
        rotation: cgmath::Quaternion<f32>,
    ) -> Option<u64> {
        let mut prefab_handle = None;
        self.prefabs
            .entry(prefab_name.to_string())
            .and_modify(|prefab| {
                let transform = InstanceTransform {
                    position,
                    rotation
                };
                transform.hash(&mut self.hasher);
                let value = self.hasher.finish();
                prefab
                    .transforms.insert(value, transform);
                prefab_handle = Some(value);
            });

        prefab_handle
    }

    pub fn prefab_instance(
        &mut self,
        prefab_name: &str,
        instance_handle: u64,
    ) -> Option<&mut InstanceTransform> {
        if let Some(prefab) = self.prefabs.get_mut(prefab_name) {
            return prefab.transforms.get_mut(&instance_handle);
        }

        None
    }

    pub fn destroy_instance(&mut self, prefab_name: &str, instance_handle: u64) {
        info!("Destroying instance {instance_handle} of {prefab_name}");
        if let Some(prefab) = self.prefabs.get_mut(prefab_name) {
            prefab.transforms.remove(&instance_handle);
        }
    }

    pub fn camera(&mut self) -> &mut Camera {
        &mut self.camera_state.camera
    }
}
