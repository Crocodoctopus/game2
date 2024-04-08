#version 430

layout(location = 0) in vec2 vert_xy;

layout(location = 1) uniform mat3 view_matrix;

out vec2 frag_uv;

void main() {
    gl_Position.xyz = view_matrix * vec3(vert_xy, 1.0);
    gl_Position.zw = vec2(1.0, 1.0);
    
    ivec2 uv[4] = {
        { 0, 0 },
        { 1, 0 },
        { 1, 1 },
        { 0, 1 },
    };

    frag_uv = uv[gl_VertexID];
}
