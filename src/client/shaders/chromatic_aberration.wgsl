struct VertexInput {
    @location(0) pos: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
}

@vertex
fn vs_main(
    in: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    /*
    if in.vert_idx == 0 {
        out.clip_position = vec4(0.0, 0.0, 1.0, 1.0);
    } else if in.vert_idx == 1 {
        out.clip_position = vec4(1.0, 0.0, 1.0, 1.0);
    } else if in.vert_idx == 2 {
        out.clip_position = vec4(1.0, 1.0, 1.0, 1.0);
    } else {
        out.clip_position = vec4(0.0, 1.0, 1.0, 1.0);
    }
    */
    out.clip_position = vec4(in.pos, 1.0, 1.0);
    return out;
}

@group(0) @binding(0)
//var t_screen: texture_storage_2d<bgra8unorm, read_write>;
var t_screen: texture_2d<f32>;
@group(0) @binding(1)
var s_screen: sampler;

struct ChromaticAberration {
    r: vec2<f32>,
    g: vec2<f32>,
    b: vec2<f32>,
}

@group(1) @binding(0)
var<uniform> rgb_offset: ChromaticAberration;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let texSize: vec2<f32> = vec2<f32>(textureDimensions(t_screen));
    let texCoord: vec2<f32> = in.clip_position.xy / texSize;
    var rgba = vec4<f32>(0);
    rgba.r = textureSample(t_screen, s_screen, texCoord + rgb_offset.r).r;
    rgba.g = textureSample(t_screen, s_screen, texCoord + rgb_offset.g).g;
    //rgba.ba = textureLoad(t_screen, s_screen, texCoord * vec2(rgb_offset.b)).ba;
    // work around:
    let temp = textureSample(t_screen, s_screen, texCoord + rgb_offset.b);
    rgba.b = temp.b;
    rgba.a = temp.a;
    return rgba;
}