@group(0) @binding(0)
var<uniform> view: mat3x3<f32>;

@group(1) @binding(0)
var light_tex: texture_2d<u32>;

struct VertexInput {
    @location(0) light_xy: vec2<f32>,
    @location(1) light_uv: vec2<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> FragmentInput {
    var out: FragmentInput;
    
    out.position = vec4<f32>(view * vec3(in.light_xy, 1.0), 1.0);
    out.light_uv = in.light_uv;

    return out;
}

struct FragmentInput {
    @builtin(position) position: vec4<f32>,
    @location(0) light_uv: vec2<f32>,
}

fn light_math(i: f32) -> f32 {
    if i == 0 {
        return 0.0;
    }
    return pow(0.92, 40. - i);
}

@fragment
fn fs_main(in: FragmentInput) -> @location(0) vec4<f32> {
    /*
    let sample = vec4<f32>(textureLoad(light_tex, vec2<u32>(in.light_uv), 0));
    
    var rgba = vec4<f32>(1.); 
    rgba.r = light_math(sample.r);
    rgba.g = light_math(sample.g);
    rgba.b = light_math(sample.b);
*/
    let coord = in.light_uv;
    let off = vec2(0.5, -0.5);

    let uv00 = floor(coord - off);
    let uv11 = floor(coord + off);
    let uv10 = vec2(uv11.x, uv00.y);
    let uv01 = vec2(uv00.x, uv11.y);

    let s00 = vec3<f32>(textureLoad(light_tex, vec2<u32>(uv00), 0).xyz);
    let s11 = vec3<f32>(textureLoad(light_tex, vec2<u32>(uv11), 0).xyz);
    let s10 = vec3<f32>(textureLoad(light_tex, vec2<u32>(uv10), 0).xyz);
    let s01 = vec3<f32>(textureLoad(light_tex, vec2<u32>(uv01), 0).xyz);

    let weight = (coord - off) - floor(coord - off);
    let t0 = mix(s01, s11, weight.x);
    let t1 = mix(s00, s10, weight.x);
    let sample = mix(t0, t1, weight.y);
    
    var rgba = vec4<f32>(1.); 
    rgba.r = light_math(sample.r);
    rgba.g = light_math(sample.g);
    rgba.b = light_math(sample.b);
    return rgba;
}
