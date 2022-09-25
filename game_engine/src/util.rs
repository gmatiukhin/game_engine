/// The coordinate system in Wgpu is based on DirectX, and Metal's coordinate systems.
/// That means that in normalized device coordinates the x axis and y axis are in the range of -1.0 to +1.0, and the z axis is 0.0 to +1.0.
/// The cgmath crate is built for OpenGL's coordinate system.
/// This matrix will scale and translate the scene from OpenGL's coordinate system to WGPU's.
#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

pub fn ortho(
    left: f32,
    right: f32,
    bottom: f32,
    top: f32,
    near: f32,
    far: f32,
) -> cgmath::Matrix4<f32> {
    OPENGL_TO_WGPU_MATRIX * cgmath::ortho(left, right, bottom, top, near, far)
}

pub fn perspective(
    fovy: impl Into<cgmath::Rad<f32>>,
    aspect: f32,
    near: f32,
    far: f32,
) -> cgmath::Matrix4<f32> {
    OPENGL_TO_WGPU_MATRIX * cgmath::perspective(fovy, aspect, near, far)
}
