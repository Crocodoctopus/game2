#version 430 core

layout (location = 0) in vec3 vert_xyz;
layout (location = 1) in vec2 vert_uv;
layout (location = 2) in vec2 vert_mask_uv;

out vec2 frag_uv;
out vec2 frag_mask_uv;

layout (location = 0) uniform mat3 view;

void main() {
    gl_Position = vec4((view * vec3(vert_xyz.xy, 1.0)).xy, vert_xyz.z / 256., 1.0);
    frag_uv = vert_uv;
    frag_mask_uv = vert_mask_uv;
}

