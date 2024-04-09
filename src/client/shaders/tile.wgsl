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
    out.clip_position = vec4<f32>(camera.view_matrix * vec3(in.tile_xyz.xy, 1.0), 1.0);
    return out;
}

@group(1) @binding(0)
var t_tile_sheet: texture_2d<f32>;
@group(1) @binding(1)
var t_mask_sheet: texture_2d<f32>;
@group(1) @binding(2)
var tex_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Get texture properties (should this be a uniform?).
    let tile_tex_size: vec2<f32> = vec2<f32>(textureDimensions(t_tile_sheet));
    let mask_tex_size: vec2<f32> = vec2<f32>(textureDimensions(t_mask_sheet));

    // Early discard if mask coevers texture.
    let mask: f32 = textureSample(t_mask_sheet, tex_sampler, in.mask_uv / mask_tex_size).r; 
    if (mask == 0) {
        discard;
    }

    // Set pixel
    var rgb: vec3<f32> = textureSample(t_tile_sheet, tex_sampler, in.tile_uv / tile_tex_size).rgb;
    if (all(rgb == vec3(1.0, 0.0, 1.0))) {
        discard;
    }
    //rgb *= mul_rgb;

    return vec4<f32>(rgb, 1.0);
}