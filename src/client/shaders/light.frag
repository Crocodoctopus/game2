#version 430

in vec2 frag_uv;

layout(location = 0) uniform usampler2DRect rgb_mask;

out vec3 rgb;

float fni(const float i) { 
    if (i == 0) return 0;
    return pow(0.93, 40. - i);
}

vec3 fn(const vec3 i) {
    return vec3(fni(i.r), fni(i.g), fni(i.b));
}

void main() {
/*
    // Nearest
    vec2 tex_size = textureSize(rgb_mask); 
    uvec3 s = texelFetch(rgb_mask, ivec2(tex_size * frag_uv)).xyz;
    rgb = vec3(fni(s[0]), fni(s[1]), fni(s[2]));
*/

    // Bilinear
    vec2 tex_size = textureSize(rgb_mask); 
    vec2 coord = tex_size * frag_uv;
    vec2 off = vec2(0.5, -0.5);

    vec2 uv00 = floor(coord - off);
    vec2 uv11 = floor(coord + off);
    vec2 uv10 = vec2(uv11.x, uv00.y);
    vec2 uv01 = vec2(uv00.x, uv11.y);

    vec3 s00 = vec3(texelFetch(rgb_mask, ivec2(uv00)).xyz);
    vec3 s11 = vec3(texelFetch(rgb_mask, ivec2(uv11)).xyz);
    vec3 s10 = vec3(texelFetch(rgb_mask, ivec2(uv10)).xyz);
    vec3 s01 = vec3(texelFetch(rgb_mask, ivec2(uv01)).xyz);
    
    vec2 weight = (coord - off) - floor(coord - off);
    vec3 t0 = mix(s01, s11, weight.x);
    vec3 t1 = mix(s00, s10, weight.x);
    vec3 final = mix(t0, t1, weight.y);
    rgb = fn(final);
}
