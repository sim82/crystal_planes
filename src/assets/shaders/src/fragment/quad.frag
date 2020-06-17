#version 450

layout(location = 0) in VertexData {
    vec4 pos;
    vec4 color;
} vertex;

layout(location = 0) out vec4 out_color;

void main() {
    vec4 color = vertex.color;

    out_color = color;
}
