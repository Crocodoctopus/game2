#version 430

in vec2 frag_tile_uv;
in vec2 frag_mask_uv;

layout(binding = 0) uniform sampler2D tile_sheet;
layout(binding = 1) uniform isampler2D mask_sheet;
layout(binding = 2) uniform vec3 mul_rgb;

out vec3 rgb;

void main() {
    // Get texture properties (should this be a uniform?).
    const vec2 tile_tex_size = vec2(textureSize(tile_sheet, 0));
    const vec2 mask_tex_size = vec2(textureSize(mask_sheet, 0));
   
    // Early discard if mask coevers texture.
    const uint mask = texture(mask_sheet, frag_mask_uv/mask_tex_size)[0]; 
    if (mask == 0) discard;

    // Set pixel.
    rgb = texture(tile_sheet, frag_tile_uv/tile_tex_size).rgb;
    if (rgb == vec3(1.0, 0.0, 1.0)) discard;
    rgb *= mul_rgb; 
}
