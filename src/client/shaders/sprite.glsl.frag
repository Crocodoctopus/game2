#version 430 core

in vec2 frag_uv;
in vec3 frag_rgb;

out vec4 rgba;

layout (location = 2) uniform sampler2D tex;

void main() {
    /*rgba = texture(tex, frag_uv);
    if (rgba.rgb == vec3(1.0, 0.0, 1.0)) {
        discard;
    }*/
    rgba = vec4(1.0, 1.0, 1.0, 1.0);
    rgba.rgb *= frag_rgb;
}
