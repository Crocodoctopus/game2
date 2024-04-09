@group(0) @binding(0)
var<uniform> view: mat3x3<f32>;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) texture_uv: vec2<f32>,
    @location(2) mask_uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) texture_uv: vec2<f32>,
    @location(1) mask_uv: vec2<f32>,
}

@vertex
fn vs_main(
    in: VertexInput
) -> VertexOutput {
    var out: VertexOutput;

    var t0: vec3<f32> = view * vec3<f32>(in.position.xy, 1.0);
    out.position.x = t0.x;
    out.position.y = t0.y;
    out.position.z = in.position.z / 256.; 
    out.position.w = 1.0; 

    out.texture_uv = in.texture_uv;
    out.mask_uv = in.mask_uv;

    return out;
}

@group(1) @binding(1)
var tile_sheet: texture_2d<f32>;
@group(1) @binding(2)
var tile_sampler: sampler;
@group(1) @binding(3)
var mask_sheet: texture_2d<f32>;
@group(1) @binding(4)
var mask_sampler: sampler;

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return vec4<f32>(1.0, 0.0, 0.0, 1.0);
}
