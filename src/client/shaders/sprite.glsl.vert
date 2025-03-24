#version 430 core

layout (location = 0) in vec3 vert_xyz;
layout (location = 1) in vec2 vert_uv;
layout (location = 2) in vec3 vert_rgb;

out vec2 frag_uv;
out vec3 frag_rgb;

layout (location = 0) uniform mat3 view;
layout (location = 1) uniform mat3 model;

void main() {
    vec3 pos = model * view * vec3(vert_xyz.xy, 1);
    gl_Position = vec4(pos.xy, vert_xyz.z, 1.0);
    frag_uv = vert_uv;
    frag_rgb = vert_rgb;
}
