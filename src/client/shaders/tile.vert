#version 430

layout(location = 0) in vec3 vert_tile_xyz;
layout(location = 1) in vec2 vert_tile_uv;
layout(location = 2) in vec2 vert_mask_uv;

out vec2 frag_tile_uv;
out vec2 frag_mask_uv;

layout(location = 3) uniform mat3 view_matrix;

void main() {
    gl_Position.xyz = view_matrix * vec3(vert_tile_xyz.xy, 1.0);
    gl_Position.zw = vec2(0 /*vert_tile_xyz.z*/ / 256., 1.0);

    frag_tile_uv = vert_tile_uv;
    frag_mask_uv = vert_mask_uv;
}
