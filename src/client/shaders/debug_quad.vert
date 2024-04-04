#version 430

layout(location = 0) in vec2 vert_xy;
layout(location = 1) in vec3 vert_rgb;

layout(location = 0) out vec3 frag_rgb;

layout(location = 0) uniform mat3 view_matrix;

void main() {
    gl_Position.xyz = view_matrix * vec3(vert_xy, 1.0);
    gl_Position.zw = vec2(0.0, 1.0);
    
    frag_rgb = vert_rgb;
}

