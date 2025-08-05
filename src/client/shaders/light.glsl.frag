#version 430 core

layout (location = 3) uniform usampler2D light_tex;

in vec2 frag_uv;

out vec4 rgba;

float light_func(float i) {
    if (i <= 0.00) {
        return 0.0;
    }
    //return pow(0.85, 40. - i);
    float cutoff = 0.94;
    return (pow(cutoff, 40. - i) - pow(cutoff, 40.)) / (pow(cutoff, 0) - pow(cutoff, 40)); 
}


void main() {
    vec2 coord = frag_uv;
    vec2 offset = vec2(0.5, -0.5);

    vec2 uv00 = floor(coord - offset);
    vec2 uv11 = floor(coord + offset);
    vec2 uv10 = vec2(uv11.x, uv00.y);
    vec2 uv01 = vec2(uv00.x, uv11.y);

    vec4 s00 = vec4(texelFetch(light_tex, ivec2(uv00), 0));
    vec4 s11 = vec4(texelFetch(light_tex, ivec2(uv11), 0));
    vec4 s10 = vec4(texelFetch(light_tex, ivec2(uv10), 0));
    vec4 s01 = vec4(texelFetch(light_tex, ivec2(uv01), 0));

    vec2 weight = (coord - offset) - floor(coord - offset);
    vec4 t0 = mix(s01, s11, weight.x);
    vec4 t1 = mix(s00, s10, weight.x);
    vec4 sf = mix(t0, t1, weight.y);

    rgba.r = light_func(sf.r);
    rgba.g = light_func(sf.g);
    rgba.b = light_func(sf.b);
    rgba.a = 1.0;
}
