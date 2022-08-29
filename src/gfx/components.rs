use crate::gfx::material::{Material, Shader, Texture};
use cgmath::EuclideanSpace;
use log::info;
use std::collections::HashMap;
use std::ops::Range;
use wgpu::util::DeviceExt;

#[derive(Debug)]
pub struct Vertex {
    pub position: cgmath::Point3<f32>,
    /// In wgpu's coordinate system UV origin is situated in the top left corner
    pub texture_coordinates: [f32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct VertexRaw {
    position: [f32; 3],
    texture_coordinates: [f32; 2],
}

impl VertexRaw {
    pub(crate) fn format<'a>() -> wgpu::VertexBufferLayout<'a> {
        const ATTRIBS: [wgpu::VertexAttribute; 2] =
            wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2];
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &ATTRIBS,
        }
    }
}

impl From<&Vertex> for VertexRaw {
    fn from(v: &Vertex) -> Self {
        Self {
            position: [v.position.x, v.position.y, v.position.z],
            texture_coordinates: v.texture_coordinates,
        }
    }
}

#[derive(Debug)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

impl Mesh {
    pub(crate) fn buffer(&self, device: &wgpu::Device) -> MeshBuffered {
        let v_vec_raw: Vec<VertexRaw> = self.vertices.iter().map(|el| el.into()).collect();

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&v_vec_raw),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&self.indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        MeshBuffered {
            vertex_buffer,
            indices_len: self.indices.len(),
            index_buffer,
        }
    }
}

pub(crate) struct MeshBuffered {
    pub(crate) vertex_buffer: wgpu::Buffer,

    pub(crate) indices_len: usize,
    pub(crate) index_buffer: wgpu::Buffer,
}

impl MeshBuffered {
    pub(crate) fn render<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        instances: Range<u32>,
    ) {
        info!("Rendering mesh");
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..self.indices_len as u32, 0, instances);
    }
}

pub struct Model {
    pub name: String,
    pub mesh: Mesh,
    pub material: Option<Material>,
    pub shader: Option<Shader>,
}

impl Model {
    pub fn new(name: &str, mesh: Mesh, material: Option<Material>, shader: Option<Shader>) -> Self {
        Self {
            name: name.to_string(),
            mesh,
            material,
            shader,
        }
    }

    pub(crate) fn buffer(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> ModelBuffered {
        let texture = if let Some(material) = &self.material {
            material.texture(device, queue)
        } else {
            Texture::default_texture(device, queue)
        };

        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: texture_bind_group_layout,
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

        let shader_module = if let Some(shader) = &self.shader {
            Some(device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(&shader.name),
                source: wgpu::ShaderSource::Wgsl((&shader.contents).into()),
            }))
        } else {
            None
        };

        ModelBuffered {
            name: self.name.clone(),
            mesh: self.mesh.buffer(&device),
            texture_bind_group,
            shader_module,
        }
    }
}

pub(crate) struct ModelBuffered {
    pub(crate) name: String,
    pub(crate) mesh: MeshBuffered,
    pub(crate) texture_bind_group: wgpu::BindGroup,
    pub(crate) shader_module: Option<wgpu::ShaderModule>,
}

impl ModelBuffered {
    pub(crate) fn render<'a>(
        &'a self,
        render_pass: &mut wgpu::RenderPass<'a>,
        instances: Range<u32>,
    ) {
        info!("Rendering model: {}", self.name);
        render_pass.set_bind_group(1, &self.texture_bind_group, &[]);
        self.mesh.render(render_pass, instances);
    }
}

pub(crate) struct Prefab {
    pub(crate) name: String,
    pub(crate) model: ModelBuffered,
    pub(crate) transforms: HashMap<usize, InstanceTransform>,
    pub(crate) instance_buffer: Option<wgpu::Buffer>,
}

impl Prefab {
    pub(crate) fn add_instance(
        &mut self,
        position: &cgmath::Point3<f32>,
        rotation: &cgmath::Quaternion<f32>,
    ) -> PrefabInstance {
        self.transforms.insert(
            self.transforms.len(),
            InstanceTransform {
                position: position.clone(),
                rotation: rotation.clone(),
            },
        );

        PrefabInstance {
            name: self.name.to_string(),
            hash: self.transforms.len() - 1,
            position: position.clone(),
            rotation: rotation.clone(),
        }
    }

    pub(crate) fn update_instance(&mut self, instance: &PrefabInstance) {
        self.transforms
            .entry(instance.hash)
            .and_modify(|instance_transform| {
                instance_transform.position = instance.position;
                instance_transform.rotation = instance.rotation;
            });
    }

    pub(crate) fn remove_instance(&mut self, instance: &PrefabInstance) {
        self.transforms.remove(&instance.hash);
    }

    pub(crate) fn update_buffer(&mut self, device: &wgpu::Device) {
        info!("Updating buffer of {}", self.name);
        let instance_data: Vec<_> = self
            .transforms
            .iter()
            .map(|(_, transform)| transform.as_raw())
            .collect();

        self.instance_buffer = Some(
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{}'s instance buffer", self.name)),
                contents: bytemuck::cast_slice(&instance_data),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            }),
        );
    }

    pub(crate) fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        if !self.transforms.is_empty() {
            info!("Rendering prefab: {}", self.name);
            if let Some(instance_buffer) = &self.instance_buffer {
                render_pass.set_vertex_buffer(1, instance_buffer.slice(..));
                self.model
                    .render(render_pass, 0..self.transforms.len() as u32);
            }
        }
    }
}

pub struct PrefabInstance {
    pub name: String,
    pub(crate) hash: usize,
    pub position: cgmath::Point3<f32>,
    pub rotation: cgmath::Quaternion<f32>,
}

#[derive(Debug)]
pub struct InstanceTransform {
    pub position: cgmath::Point3<f32>,
    pub rotation: cgmath::Quaternion<f32>,
}

impl InstanceTransform {
    pub(crate) fn as_raw(&self) -> InstanceTransformRaw {
        info!("Transforming Instance into InstanceTransformRaw");
        InstanceTransformRaw {
            translation: (cgmath::Matrix4::from_translation(self.position.to_vec())
                * cgmath::Matrix4::from(self.rotation))
            .into(),
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct InstanceTransformRaw {
    translation: [[f32; 4]; 4],
}

impl InstanceTransformRaw {
    pub(crate) fn format<'a>() -> wgpu::VertexBufferLayout<'a> {
        const ATTRIBS: [wgpu::VertexAttribute; 4] = wgpu::vertex_attr_array![2 => Float32x4, 3 => Float32x4, 4 => Float32x4, 5 => Float32x4];
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &ATTRIBS,
        }
    }
}
