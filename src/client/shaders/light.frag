#version 430

in vec2 frag_uv;

layout(location = 0) uniform usampler2D rgb_mask;

out vec3 rgb;

float fn(const uint i) { 
    if (i == 0) return 0;
    return pow(0.90, 31 - i);
}

void main() {
    rgb.r = fn(texture(rgb_mask, frag_uv).r);
    rgb.g = fn(texture(rgb_mask, frag_uv).g);
    rgb.b = fn(texture(rgb_mask, frag_uv).b);
}
