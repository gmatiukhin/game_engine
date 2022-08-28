use cgmath::*;
use wgpu::util::DeviceExt;

use crate::util::OPENGL_TO_WGPU_MATRIX;

#[derive(Debug)]
pub struct Camera {
    pub position: Point3<f32>,
    pub yaw: Rad<f32>,
    pub pitch: Rad<f32>,
    pub aspect: f32,
    pub fovy: Rad<f32>,
    pub z_near: f32,
    pub z_far: f32,
}

impl Camera {
    pub fn new<V: Into<Point3<f32>>, Y: Into<Rad<f32>>, P: Into<Rad<f32>>, F: Into<Rad<f32>>>(
        position: V,
        yaw: Y,
        pitch: P,
        width: u32,
        height: u32,
        fovy: F,
        z_near: f32,
        z_far: f32,
    ) -> Self {
        Self {
            position: position.into(),
            yaw: yaw.into(),
            pitch: pitch.into(),
            aspect: width as f32 / height as f32,
            fovy: fovy.into(),
            z_near,
            z_far,
        }
    }

    /// AKA world-to-camera matrix
    pub(crate) fn calc_view_matrix(&self) -> Matrix4<f32> {
        Matrix4::look_to_rh(self.position, self.view_direction(), self.up_direction())
    }

    pub(crate) fn calc_projection(&self) -> Matrix4<f32> {
        // perspective() returns right-handed projection matrix
        OPENGL_TO_WGPU_MATRIX * perspective(self.fovy, self.aspect, self.z_near, self.z_far)
    }

    pub(crate) fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
    }

    /// Forward direction without pitch
    pub fn forward_direction(&self) -> Vector3<f32> {
        let (yaw_sin, yaw_cos) = self.yaw.sin_cos();
        Vector3::new(yaw_sin, 0.0, -yaw_cos).normalize()
    }

    pub fn view_direction(&self) -> Vector3<f32> {
        Vector3 {
            x: self.yaw.sin(),
            y: -self.pitch.sin(),
            z: -self.yaw.cos(),
        }
        .normalize()
    }

    pub fn right_direction(&self) -> Vector3<f32> {
        let (yaw_sin, yaw_cos) = self.yaw.sin_cos();
        Vector3::new(yaw_cos, 0.0, yaw_sin).normalize()
    }

    pub fn up_direction(&self) -> Vector3<f32> {
        Vector3::unit_y()
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    view_position: [f32; 4],
    // Can't use cgmath with bytemuck directly.
    // Need to convert the Matrix4 into a 4x4 f32 array
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        Self {
            view_position: [0.0; 4],
            view_proj: Matrix4::identity().into(),
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera) {
        self.view_position = camera.position.to_homogeneous().into();
        self.view_proj = (camera.calc_projection() * camera.calc_view_matrix()).into();
    }
}

pub(crate) struct CameraState {
    pub(crate) camera: Camera,
    pub(crate) camera_uniform: CameraUniform,
    pub(crate) camera_buffer: wgpu::Buffer,
    pub(crate) camera_bind_group_layout: wgpu::BindGroupLayout,
    pub(crate) camera_bind_group: wgpu::BindGroup,
}

impl CameraState {
    pub(crate) fn default_state(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
    ) -> Self {
        let camera = Camera::new(
            (0.0, 0.0, 0.0),
            Deg(0.0),
            Deg(0.0),
            config.width,
            config.height,
            Deg(90.0),
            0.1,
            100.0,
        );

        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&camera);
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("camera_buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("camera_bind_group_layout"),
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

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera_bind_group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        Self {
            camera,
            camera_uniform,
            camera_buffer,
            camera_bind_group_layout,
            camera_bind_group,
        }
    }

    pub(crate) fn update(&mut self, queue: &wgpu::Queue) {
        self.camera_uniform.update_view_proj(&self.camera);
        queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );
    }
}
