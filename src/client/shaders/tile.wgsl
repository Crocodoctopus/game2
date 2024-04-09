struct CameraUniform {
    view_matrix: mat3x3<f32>,
};
@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct FragmentInput {
    @location(0) tile_xyz: vec3<f32>,
    @location(1) tile_uv: vec2<f32>,
    @location(2) mask_uv: vec2<f32>,
};

struct VertexOutput {
    // TOOD: review location stuff for frag
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tile_uv: vec2<f32>,
    @location(1) mask_uv: vec2<f32>,
}

@vertex
fn vs_main(
    in: FragmentInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.tile_uv = in.tile_uv;
    out.mask_uv = in.mask_uv;
    //out.clip_position = vec4<f32>(camera.view_matrix * in.tile_xyz, 1.0);
    out.clip_position = vec4<f32>(camera.view_matrix * in.tile_xyz, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(0.3, 0.2, 0.1, 1.0);
}