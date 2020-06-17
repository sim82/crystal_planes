#version 450


layout(std140, set = 0, binding = 0) uniform Projview {
    mat4 proj;
    mat4 view;
    mat4 proj_view;
};


layout(location = 0) in vec3 position;
layout(location = 1) in vec4 color;
layout(location = 2) in vec3 translate;
layout(location = 3) in uint dir;


layout(location = 0) out VertexData {
    vec4 pos;
    vec4 color;
} vertex;


void main() {
    mat4 modelx0 = mat4(0.125, 0.0, 0.0, 0.0, 0.0, 0.125, 0.0, 0.0, 0.0, 0.0, 0.125, 0.0, 0.0, 0.0, 0.125, 1.0);
    mat4 modelx1 = mat4(-0.125, 0.0, 0.0, 0.0, 0.0, 0.125, 0.0, 0.0, 0.0, 0.0, -0.125, 0.0, 0.0, 0.0, -0.125, 1.0);
    mat4 modelx2 = mat4(0.0, 0.0, -0.125, 0.0, 0.0, 0.125, 0.0, 0.0, 0.125, 0.0, 0.0, 0.0, 0.125, 0.0, 0.0, 1.0);
    mat4 modelx3 = mat4(0.0, -0.0, 0.125, 0.0, 0.0, 0.125, 0.0, 0.0, -0.125, 0.0, 0.0, 0.0, -0.125, 0.0, 0.0, 1.0);
    mat4 modelx4 = mat4(-0.125, 0.0, 0.0, 0.0, 0.0, 0.0, 0.125, 0.0, 0.0, 0.125, 0.0, 0.0, 0.0, 0.125, 0.0, 1.0);
    mat4 modelx5 = mat4(-0.125, -0.0, 0.0, 0.0, 0.0, 0.0, -0.125, 0.0, 0.0, -0.125, 0.0, 0.0, 0.0, -0.125, 0.0, 1.0);
    mat4 modelx[6] = mat4[6](modelx0, modelx1, modelx2, modelx3, modelx4, modelx5);
   
    vertex.color = color;
    mat4 trans_mat = mat4(1.0);
    trans_mat[3] = vec4(translate, 1.0);
    mat4 model2 = trans_mat * modelx[dir];
    vertex.pos = vec4(position, 1.0);
    gl_Position = proj * view * model2 * vertex.pos;

    // vertex.pos = vec4(0.0, 0.0, 0.0, 1.0);
    // vertex.color = vec4(1.0, 0.0, 0.0, 1.0);
    // gl_Position = proj_view * vertex.pos;

}
