#version 430 core

layout (location = 1) uniform sampler2D sprite_texture;
layout (location = 2) uniform isampler2D mask_texture;
layout (location = 3) uniform vec4 rgba_multiplier;

in vec2 frag_uv;
in vec2 frag_mask_uv;

out vec4 rgba;

void main() {   
    // Early discard.
    int mask = texelFetch(mask_texture, ivec2(frag_mask_uv), 0)[0];
    if (mask == 0) {
        discard;
    } 

    // Set pixel.
    vec3 rgb = texelFetch(sprite_texture, ivec2(frag_uv), 0).rgb;
    if (rgb == vec3(1.0, 0.0, 1.0)) {
        discard;
    }

    rgba = vec4(rgb, 1.0) * rgba_multiplier;
}

