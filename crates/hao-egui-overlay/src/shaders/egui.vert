#version 450
layout(push_constant) uniform PC { vec2 screen_size; };
layout(location = 0) in vec2 in_pos;
layout(location = 1) in vec2 in_uv;
layout(location = 2) in uint in_color;
layout(location = 0) out vec2 out_uv;
layout(location = 1) out vec4 out_color;

vec4 unpack_color(uint c) {
    return vec4(
        float(c & 0xFFu) / 255.0,
        float((c >> 8u) & 0xFFu) / 255.0,
        float((c >> 16u) & 0xFFu) / 255.0,
        float((c >> 24u) & 0xFFu) / 255.0
    );
}

void main() {
    gl_Position = vec4(
        2.0 * in_pos.x / screen_size.x - 1.0,
        2.0 * in_pos.y / screen_size.y - 1.0,
        0.0, 1.0
    );
    out_uv = in_uv;
    out_color = unpack_color(in_color);
}
