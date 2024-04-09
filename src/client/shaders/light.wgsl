struct CameraUniform {
    view_matrix: mat3x3<f32>,
};
@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexInput {
    @builtin(vertex_index) vert_idx: u32,
    @location(0) light_xy: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) light_uv: vec2<f32>,
}

@vertex
fn vs_main(
    in: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    //let uv = array(vec2(0, 0), vec2(1, 0), vec2(1, 1), vec2(0, 0));
    //out.light_uv = uv[in.vert_idx];
    if in.vert_idx == 0 {
        out.light_uv = vec2(0.0, 0.0);
    } else if in.vert_idx == 1 {
        out.light_uv = vec2(1.0, 0.0);
    } else if in.vert_idx == 2 {
        out.light_uv = vec2(1.0, 1.0);
    } else {
        out.light_uv = vec2(0.0, 0.0);
    }
    out.clip_position = vec4<f32>(camera.view_matrix * vec3(in.light_xy, 1.0), 1.0);
    return out;
}

@group(1) @binding(0)
var t_rgb_mask: texture_2d<u32>;
@group(1) @binding(1)
var s_rgb_mask: sampler;

fn light_math(i: u32) -> f32 {
    if i == 0 {
        return 0.0;
    }
    return pow(0.90, 31 - f32(i));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var rgb = vec3<f32>(0);
    //let sample = textureSample(t_rgb_mask, s_rgb_mask, in.light_uv);
    let sample = textureLoad(t_rgb_mask, vec2<i32>(in.light_uv), 1);
    rgb.r = light_math(sample.r);
    rgb.g = light_math(sample.g);
    rgb.b = light_math(sample.b);

    return vec4<f32>(rgb, 1.0);
}