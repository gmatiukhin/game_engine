use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use cgmath::EuclideanSpace;
use log::{info, warn};
use wgpu::util::DeviceExt;

pub struct Vertex {
    pub position: cgmath::Point3<f32>,
    pub color: wgpu::Color,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct VertexRaw {
    position: [f32; 3],
    color: [f32; 4],
}

impl VertexRaw {
    pub(crate) fn format<'a>() -> wgpu::VertexBufferLayout<'a> {
        const ATTRIBS: [wgpu::VertexAttribute; 2] =
            wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4];
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
            color: [
                v.color.r as f32,
                v.color.g as f32,
                v.color.b as f32,
                v.color.a as f32,
            ],
        }
    }
}

pub struct Mesh {
    pub name: String,

    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

impl Mesh {
    pub(crate) fn as_raw(&self, device: &wgpu::Device) -> MeshRaw {
        info!("Transforming Mesh into MeshRaw");
        let mut v_buffer: Option<wgpu::Buffer> = None;
        if !self.vertices.is_empty() {
            let v_vec_raw: Vec<VertexRaw> = self.vertices.iter().map(|el| el.into()).collect();
            v_buffer = Some(
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("{}'s vertex buffer", self.name)),
                    contents: bytemuck::cast_slice(&v_vec_raw),
                    usage: wgpu::BufferUsages::VERTEX,
                }),
            );
        } else {
            warn!("Empty vertex buffer of {}", self.name);
        }

        let mut i_buffer: Option<wgpu::Buffer> = None;
        if !self.indices.is_empty() {
            i_buffer = Some(
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("{}'s index buffer", self.name)),
                    contents: bytemuck::cast_slice(&self.indices),
                    usage: wgpu::BufferUsages::INDEX,
                }),
            );
        } else {
            warn!("Empty index buffer of {}", self.name);
        }

        MeshRaw {
            name: self.name.clone(),
            vertex_buffer: v_buffer,
            vertices_length: self.vertices.len() as u32,
            index_buffer: i_buffer,
            indices_length: self.indices.len() as u32,
        }
    }
}

pub(crate) struct MeshRaw {
    name: String,

    vertex_buffer: Option<wgpu::Buffer>,
    vertices_length: u32,

    index_buffer: Option<wgpu::Buffer>,
    indices_length: u32,
}

impl MeshRaw {
    pub(crate) fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        info!("Rendering mesh: {}", self.name);

        if let Some(v_buffer) = &self.vertex_buffer {
            render_pass.set_vertex_buffer(0, v_buffer.slice(..));
        }

        if let Some(i_buffer) = &self.index_buffer {
            render_pass.set_index_buffer(i_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..self.indices_length, 0, 0..1);
        } else {
            render_pass.draw(0..self.vertices_length, 0..1);
        }
    }

    pub(crate) fn render_instanced<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>, instances: std::ops::Range<u32>) {
        info!("Rendering instances of {}", self.name);

        if let Some(v_buffer) = &self.vertex_buffer {
            render_pass.set_vertex_buffer(0, v_buffer.slice(..));
        }

        if let Some(i_buffer) = &self.index_buffer {
            render_pass.set_index_buffer(i_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.draw_indexed(0..self.indices_length, 0, instances);
        } else {
            render_pass.draw(0..self.vertices_length, instances);
        }
    }
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
            translation: (cgmath::Matrix4::from_translation(self.position.to_vec()) * cgmath::Matrix4::from(self.rotation)).into(),
        }
    }
}

impl Hash for InstanceTransform {
    fn hash<H: Hasher>(&self, state: &mut H) {
        format!("{:?}", self).hash(state);
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct InstanceTransformRaw {
    translation: [[f32; 4]; 4],
}

impl InstanceTransformRaw {
    pub(crate) fn format<'a>() -> wgpu::VertexBufferLayout<'a> {
        const ATTRIBS: [wgpu::VertexAttribute; 4] =
            wgpu::vertex_attr_array![2 => Float32x4, 3 => Float32x4, 4 => Float32x4, 5 => Float32x4];
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &ATTRIBS,
        }
    }
}

pub(crate) struct Prefab {
    pub(crate) name: String,
    pub(crate) mesh: MeshRaw,
    pub(crate) transforms: HashMap<u64, InstanceTransform>,
    pub(crate) instance_buffer: Option<wgpu::Buffer>,
}

impl Prefab {
    pub(crate) fn update_buffer(&mut self, device: &wgpu::Device) {
        info!("Updating buffer of {}", self.name);
        let instance_data: Vec<_> = self.transforms.iter().map(|(_, transform)| transform.as_raw()).collect();
        info!("{}'s instance data array: {:?}", self.name, instance_data);
        self.instance_buffer = Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{}'s instance buffer", self.name)),
            contents: bytemuck::cast_slice(&instance_data),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        }));
    }

    pub(crate) fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        if !self.transforms.is_empty() {
            info!("Rendering prefab: {}", self.name);
            if let Some(instances) = &self.instance_buffer {
                render_pass.set_vertex_buffer(1, instances.slice(..));
                self.mesh.render_instanced(render_pass, 0..self.transforms.len() as u32);
            }
        }
    }
}
