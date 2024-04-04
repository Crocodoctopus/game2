#version 430

layout(location = 0) in vec3 frag_rgb;

layout(location = 0) out vec3 rgb;

void main() {
    rgb = frag_rgb;
}

