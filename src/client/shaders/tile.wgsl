@group(0) @binding(0)
var<uniform> view: mat3x3<f32>;
@group(0) @binding(1)
var generic_sampler: sampler;

@group(1) @binding(0)
var sprite_tex: texture_2d<f32>;
@group(1) @binding(1)
var mask_tex: texture_2d<u32>;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) texture_uv: vec2<f32>,
    @location(2) mask_uv: vec2<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> FragmentInput {
    var out: FragmentInput;

    var t0: vec3<f32> = view * vec3<f32>(in.position.xy, 1.0);
    out.position = vec4<f32>(t0.xy, in.position.z / 256., 1.0);
    out.texture_uv = in.texture_uv;
    out.mask_uv = in.mask_uv;

    return out;
}

struct FragmentInput {
    @builtin(position) position: vec4<f32>,
    @location(0) texture_uv: vec2<f32>,
    @location(1) mask_uv: vec2<f32>,
}

@fragment
fn fs_main(in: FragmentInput) -> @location(0) vec4<f32> {
    // Get texture properties (should this be a uniform?).
    //let sprite_tex_size: vec2<f32> = vec2<f32>(textureDimensions(sprite_tex));
    //let mask_tex_size: vec2<f32> = vec2<f32>(textureDimensions(mask_tex));

    // Early discard if mask coevers texture.
    let mask: u32 = textureLoad(mask_tex, vec2<u32>(in.mask_uv), 0)[0];
    if (mask == 0) {
        discard;
    }

    // Set pixel
    var rgb = textureLoad(sprite_tex, vec2<u32>(in.texture_uv), 0).rgb;
    if (all(rgb == vec3(1.0, 0.0, 1.0))) {
        discard;
    }

    return vec4<f32>(vec3<f32>(rgb), 1.0);
}
