#version 430 core

layout (location = 0) uniform mat3 view;
layout (location = 1) uniform vec4 bounds;
layout (location = 2) uniform vec2 tex_size;

vec2 offset[] = {
    vec2(0.0, 0.0),
    vec2(1.0, 0.0),
    vec2(1.0, 1.0),
    vec2(0.0, 1.0),
};

out vec2 frag_uv;

void main() {
    float x = bounds.x + bounds[2] * offset[gl_VertexID][0];
    float y = bounds.y + bounds[3] * offset[gl_VertexID][1];
    gl_Position = vec4(view * vec3(x, y, 1), 1); 
    float u = tex_size[0] * offset[gl_VertexID][0];
    float v = tex_size[1] * offset[gl_VertexID][1];
    frag_uv = vec2(u, v);
}
