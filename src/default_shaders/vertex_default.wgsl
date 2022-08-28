struct VertexInput {
    @location(0) world_position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
}

struct CameraUniform {
    // position: vec3<f32>,
    view_proj: mat4x4<f32>,
}

@group(0) @binding(0) var<uniform> camera: CameraUniform;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(in.world_position, 1.0);
    out.world_position = in.world_position;
    out.tex_coords = in.tex_coords;
    return out;
}