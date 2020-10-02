use std::sync::{mpsc::Receiver, Mutex};

use crate::crystal::{self, rad};
use bevy::{
    diagnostic::{Diagnostic, DiagnosticId, Diagnostics},
    prelude::*,
    render::{
        mesh::{shape, VertexAttributeValues},
        pipeline::{DynamicBinding, PipelineDescriptor, PipelineSpecialization, RenderPipeline},
        render_graph::{base, AssetRenderResourcesNode, RenderGraph},
        renderer::RenderResources,
        shader::{ShaderStage, ShaderStages},
    },
};
use rand::{thread_rng, Rng};
#[derive(RenderResources, Default)]
struct MyMaterial {
    pub color: Color,
}
const VERTEX_SHADER: &str = r#"
#version 450
layout(location = 0) in vec3 Vertex_Position;
layout(location = 1) in vec3 Vertex_Normal;
layout(set = 0, binding = 0) uniform Camera {
    mat4 ViewProj;
};
layout(set = 1, binding = 0) uniform Transform {
    mat4 Model;
};

layout(location = 0) out vec4 Vertex_Color;

void main() {
    gl_Position = ViewProj * Model * vec4(Vertex_Position, 1.0);
    Vertex_Color = vec4(Vertex_Normal, 1.0);
}
"#;

const FRAGMENT_SHADER: &str = r#"
#version 450
layout(location = 0) out vec4 o_Target;
layout(set = 1, binding = 1) uniform MyMaterial_color {
    vec4 color;
};
layout(location = 0) in vec4 Vertex_Color;

void main() {
    o_Target = Vertex_Color;
    //o_Target = color;
}
"#;
#[derive(Bundle)]
struct PlaneComponents {
    plane: Plane,
}
struct Plane {
    mesh_handle: Handle<Mesh>, // the mesh that contains the plane
    indices: [u32; 4], // indices of the attributes that belong to this plane in 'mesh_handle'
}

fn setup(
    mut commands: Commands,
    mut pipelines: ResMut<Assets<PipelineDescriptor>>,
    mut shaders: ResMut<Assets<Shader>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<MyMaterial>>,
    mut render_graph: ResMut<RenderGraph>,
    plane_scene: Res<crystal::PlaneScene>,
    mut query: Query<(Entity, &rad::Plane)>,
) {
    // Create a new shader pipeline
    let pipeline_handle = pipelines.add(PipelineDescriptor::default_config(ShaderStages {
        vertex: shaders.add(Shader::from_glsl(ShaderStage::Vertex, VERTEX_SHADER)),
        fragment: Some(shaders.add(Shader::from_glsl(ShaderStage::Fragment, FRAGMENT_SHADER))),
    }));

    // Add an AssetRenderResourcesNode to our Render Graph. This will bind MyMaterial resources to our shader
    render_graph.add_system_node(
        "my_material",
        AssetRenderResourcesNode::<MyMaterial>::new(true),
    );

    // Add a Render Graph edge connecting our new "my_material" node to the main pass node. This ensures "my_material" runs before the main pass
    render_graph
        .add_node_edge("my_material", base::node::MAIN_PASS)
        .unwrap();

    let pipelines = RenderPipelines::from_pipelines(vec![RenderPipeline::specialized(
        pipeline_handle,
        // NOTE: in the future you wont need to manually declare dynamic bindings
        PipelineSpecialization {
            dynamic_bindings: vec![
                // Transform
                DynamicBinding {
                    bind_group: 1,
                    binding: 0,
                },
                // MyMaterial_color
                DynamicBinding {
                    bind_group: 1,
                    binding: 1,
                },
            ],
            ..Default::default()
        },
    )]);

    // Setup our world
    commands
        .spawn(Camera3dComponents {
            transform: Transform::new(Mat4::face_toward(
                Vec3::new(5.0, 5.0, 20.0),
                Vec3::new(5.0, 5.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            )),
            ..Default::default()
        })
        .with(bevy_fly_camera::FlyCamera {
            mouse_drag: true,
            sensitivity: 8.0,
            ..Default::default()
        });

    let mut meshes_tmp = Vec::new();
    for p in plane_scene.planes.planes_iter() {
        let point = &p.cell;
        let plane_trans = match p.dir {
            crystal::Dir::XyPos => Mat4::from_cols_array(&[
                0.125, 0.0, 0.0, 0.0, 0.0, 0.125, 0.0, 0.0, 0.0, 0.0, 0.125, 0.0, 0.0, 0.0, 0.125,
                1.0,
            ]),
            crystal::Dir::XyNeg => Mat4::from_cols_array(&[
                -0.125, 0.0, 0.0, 0.0, 0.0, 0.125, 0.0, 0.0, 0.0, 0.0, -0.125, 0.0, 0.0, 0.0,
                -0.125, 1.0,
            ]),
            crystal::Dir::YzPos => Mat4::from_cols_array(&[
                0.0, 0.0, -0.125, 0.0, 0.0, 0.125, 0.0, 0.0, 0.125, 0.0, 0.0, 0.0, 0.125, 0.0, 0.0,
                1.0,
            ]),
            crystal::Dir::YzNeg => Mat4::from_cols_array(&[
                0.0, -0.0, 0.125, 0.0, 0.0, 0.125, 0.0, 0.0, -0.125, 0.0, 0.0, 0.0, -0.125, 0.0,
                0.0, 1.0,
            ]),
            crystal::Dir::ZxPos => Mat4::from_cols_array(&[
                -0.125, 0.0, 0.0, 0.0, 0.0, 0.0, 0.125, 0.0, 0.0, 0.125, 0.0, 0.0, 0.0, 0.125, 0.0,
                1.0,
            ]),
            crystal::Dir::ZxNeg => Mat4::from_cols_array(&[
                -0.125, -0.0, 0.0, 0.0, 0.0, 0.0, -0.125, 0.0, 0.0, -0.125, 0.0, 0.0, 0.0, -0.125,
                0.0, 1.0,
            ]),
        };
        // println!("spawn");

        meshes_tmp.push((
            Mesh::from(shape::Quad {
                size: Vec2::new(2.0, 2.0),
                flip: false,
            }),
            Mat4::from_translation(point.into_vec3() * 0.25) * plane_trans,
        ));
    }

    let mut plane_entities = std::collections::HashMap::<usize, Entity>::new();
    for (ent, plane) in &mut query.iter() {
        plane_entities.insert(plane.buf_index, ent);
    }

    let mut num_planes = 0;
    let mut spawn_mesh = {
        // FIXME: why is type inference for 'planes' broken?
        |position, normal, uv, index, planes: Vec<u32>| {
            let mesh = Mesh {
                primitive_topology: bevy::render::pipeline::PrimitiveTopology::TriangleList,
                attributes: vec![
                    bevy::render::mesh::VertexAttribute::position(position),
                    bevy::render::mesh::VertexAttribute::normal(normal),
                    bevy::render::mesh::VertexAttribute::uv(uv),
                ],
                indices: Some(index),
            };

            let mesh_handle = meshes.add(mesh);
            commands
                .spawn(MeshComponents {
                    mesh: mesh_handle,
                    render_pipelines: pipelines.clone(),
                    ..Default::default()
                })
                .with(materials.add(MyMaterial {
                    color: Color::rgb(0.0, 0.0, 1.0),
                }));

            for p in planes.iter() {
                // glue local Plane component (ToDo: rename) to pre-existing 'plane' entities
                commands.insert(
                    *plane_entities
                        .get(&num_planes)
                        .expect("missing entity for plane index"),
                    PlaneComponents {
                        plane: Plane {
                            mesh_handle,
                            indices: [p + 0, p + 1, p + 2, p + 3],
                        },
                    },
                );
                num_planes += 1;
            }
        }
    };

    let mut position = Vec::new();
    let mut normal = Vec::new();
    let mut uv = Vec::new();
    let mut index = Vec::new();
    let mut planes = Vec::new();
    for (mesh, trans) in meshes_tmp.iter() {
        if position.len() > 256 * 256 - 4 {
            spawn_mesh(position, normal, uv, index, planes);

            position = Vec::new();
            normal = Vec::new();
            uv = Vec::new();
            index = Vec::new();
            planes = Vec::new();
        }

        let index_offset = position.len() as u32;
        planes.push(index_offset); // this is also the first index that belongs to the current plane
        for attribute in &mesh.attributes {
            if attribute.name == "Vertex_Position" {
                match &attribute.values {
                    VertexAttributeValues::Float3(vs) => {
                        for v in vs {
                            let v: Vec3 = v.clone().into();
                            let v: Vec3 = (*trans * v.extend(1.0)).truncate().into();
                            position.push([v.x(), v.y(), v.z()]);
                        }
                    }
                    _ => panic!("expected Vertex_Position to be Float3"),
                }
            } else if attribute.name == "Vertex_Normal" {
                match &attribute.values {
                    VertexAttributeValues::Float3(vs) => normal.append(&mut vs.clone()),
                    _ => panic!("expected Vertex_Normal to be Float3"),
                }
            // normal.append(other)
            } else if attribute.name == "Vertex_Uv" {
                match &attribute.values {
                    VertexAttributeValues::Float2(vs) => uv.append(&mut vs.clone()),
                    _ => panic!("expected Vertex_Uv to be Float2"),
                }
            }
        }
        match &mesh.indices {
            Some(indices) => {
                index.append(&mut indices.iter().map(|i| i + index_offset).collect());
            }
            _ => panic!("expected index array"),
        }
    }
    spawn_mesh(position, normal, uv, index, planes);
}

fn _blink_system(mut meshes: ResMut<Assets<Mesh>>, plane: &Plane) {
    // println!("blink");
    let mesh = meshes
        .get_mut(&plane.mesh_handle)
        .expect("bad mesh_handle in Plane entitiy");

    for a in &mut mesh.attributes {
        if a.name == "Vertex_Normal" {
            match &mut a.values {
                VertexAttributeValues::Float3(ref mut vs) => {
                    let color =
                        Color::rgb(thread_rng().gen(), thread_rng().gen(), thread_rng().gen());
                    for i in plane.indices.iter() {
                        vs[*i as usize][0] = color.r;
                        vs[*i as usize][1] = color.g;
                        vs[*i as usize][2] = color.b;
                    }
                }
                _ => panic!("expected Vertex_Normal to be Float3"),
            }
        }
    }
}

fn apply_frontbuf(
    front_buf: Res<rad::FrontBuf>,
    mut meshes: ResMut<Assets<Mesh>>,
    rad_to_render: Res<Mutex<Receiver<rad::RadToRender>>>,
    mut diagnostics: ResMut<Diagnostics>,
    mut query: Query<(&rad::Plane, &Plane)>,
) {
    let mut update = false;
    for cmd in rad_to_render.lock().unwrap().try_iter() {
        match cmd {
            rad::RadToRender::IterationDone { num_int, duration } => {
                diagnostics
                    .add_measurement(RAD_INT_PER_SECOND, num_int as f64 / duration.as_secs_f64());
                update = true;
            }
        }
    }

    if !update {
        return;
    }
    let mut mesh_opt = None;
    let mut mesh_handle = Handle::<Mesh>::default();
    for (rad_plane, plane) in &mut query.iter() {
        // read rgb value from rad frontbuffer
        let buf = front_buf.read();
        let r = buf.r[rad_plane.buf_index];
        let g = buf.g[rad_plane.buf_index];
        let b = buf.b[rad_plane.buf_index];

        // consecutive planes will mostly be located in the same mesh
        if mesh_handle != plane.mesh_handle {
            mesh_handle = plane.mesh_handle;
            mesh_opt = meshes.get_mut(&plane.mesh_handle);
        }

        if let Some(ref mut mesh) = mesh_opt {
            for a in &mut mesh.attributes {
                if a.name == "Vertex_Normal" {
                    match &mut a.values {
                        VertexAttributeValues::Float3(ref mut vs) => {
                            for i in plane.indices.iter() {
                                vs[*i as usize][0] = r;
                                vs[*i as usize][1] = g;
                                vs[*i as usize][2] = b;
                            }
                        }
                        _ => panic!("expected Vertex_Normal to be Float3"),
                    }
                }
            }
        }
    }
}

pub const RAD_INT_PER_SECOND: DiagnosticId =
    DiagnosticId::from_u128(337040787172757619024841343456040760896);

fn setup_diagnostic_system(mut diagnostics: ResMut<Diagnostics>) {
    // Diagnostics must be initialized before measurements can be added.
    // In general it's a good idea to set them up in a "startup system".
    diagnostics.add(Diagnostic::new(
        RAD_INT_PER_SECOND,
        "rad_int_per_second",
        10,
    ));
}

#[derive(Default)]
pub struct QuadRenderPlugin;

impl Plugin for QuadRenderPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_startup_system_to_stage("renderer", setup.system())
            .add_startup_system(setup_diagnostic_system.system())
            // .add_system(blink_system.system())
            .add_system(apply_frontbuf.system())
            .add_asset::<MyMaterial>();
    }
}
