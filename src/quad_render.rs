use crate::map;
use crate::rad;
use bevy::asset::Asset;
use bevy::render::draw::OutsideFrustum;
use bevy::render::mesh::Indices;
use bevy::render::shader::ShaderDefs;
use bevy::{
    prelude::*,
    reflect::TypeUuid,
    render::{
        mesh::{shape, VertexAttributeValues},
        pipeline::{PipelineDescriptor, RenderPipeline},
        render_graph::{base, AssetRenderResourcesNode, RenderGraph},
        renderer::RenderResources,
        shader::{ShaderStage, ShaderStages},
    },
};
use tracing::info;

pub const ATTRIBUTE_COLOR: &'static str = "Vertex_Color";

// FIXME: this is only defined here because apply_frontbuf directly needs to modify it. Implementation should be moved from main.rs
#[derive(Default)]
pub struct RotatorSystemState {
    pub run: bool,
}

// #[derive(RenderResources, Default, TypeUuid, ShaderDefs)]
// #[uuid = "213b8673-5cf1-441e-b98d-4602a612567e"]
// struct MyMaterial {}
const VERTEX_SHADER: &str = r#"
#version 450
layout(location = 0) in vec3 Vertex_Position;
layout(location = 1) in vec3 Vertex_Color;
layout(location = 0) out vec3 v_color;

layout(set = 0, binding = 0) uniform CameraViewProj {
    mat4 ViewProj;
};
layout(set = 1, binding = 0) uniform Transform {
    mat4 Model;
};


void main() {
    gl_Position = ViewProj * Model * vec4(Vertex_Position, 1.0);
    v_color =  Vertex_Color;
}
"#;

const FRAGMENT_SHADER: &str = r#"
#version 450
layout(location = 0) out vec4 o_Target;
layout(location = 0) in vec3 v_color;

void main() {
    o_Target = vec4(v_color, 1.0);
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

pub struct QuadRenderMesh;

fn setup(
    mut commands: Commands,
    mut pipelines: ResMut<Assets<PipelineDescriptor>>,
    mut shaders: ResMut<Assets<Shader>>,
    mut meshes: ResMut<Assets<Mesh>>,
    // mut materials: ResMut<Assets<MyMaterial>>,
    // mut standard_materials: ResMut<Assets<StandardMaterial>>,
    // mut render_graph: ResMut<RenderGraph>,
    plane_scene: Res<map::PlaneScene>,
    query: Query<(Entity, &rad::PlaneIndex)>,
) {
    info!("QuadRender setup");
    // Create a new shader pipeline
    let pipeline_handle = pipelines.add(PipelineDescriptor::default_config(ShaderStages {
        vertex: shaders.add(Shader::from_glsl(ShaderStage::Vertex, VERTEX_SHADER)),
        fragment: Some(shaders.add(Shader::from_glsl(ShaderStage::Fragment, FRAGMENT_SHADER))),
    }));
    // Add an AssetRenderResourcesNode to our Render Graph. This will bind MyMaterial resources to our shader
    // render_graph.add_system_node(
    //     "my_material",
    //     AssetRenderResourcesNode::<MyMaterial>::new(true),
    // );
    // Add a Render Graph edge connecting our new "my_material" node to the main pass node. This ensures "my_material" runs before the main pass
    // render_graph
    //     .add_node_edge("my_material", base::node::MAIN_PASS)
    //     .unwrap();

    // let render_pipeline = RenderPipeline::new(pipeline_handle);

    // Setup our world

    let mut meshes_tmp = Vec::new();
    for p in plane_scene.planes.planes_iter() {
        let point = &p.cell;
        let plane_trans = match p.dir {
            map::Dir::XyPos => Mat4::from_cols_array(&[
                0.125, 0.0, 0.0, 0.0, 0.0, 0.125, 0.0, 0.0, 0.0, 0.0, 0.125, 0.0, 0.0, 0.0, 0.125,
                1.0,
            ]),
            map::Dir::XyNeg => Mat4::from_cols_array(&[
                -0.125, 0.0, 0.0, 0.0, 0.0, 0.125, 0.0, 0.0, 0.0, 0.0, -0.125, 0.0, 0.0, 0.0,
                -0.125, 1.0,
            ]),
            map::Dir::YzPos => Mat4::from_cols_array(&[
                0.0, 0.0, -0.125, 0.0, 0.0, 0.125, 0.0, 0.0, 0.125, 0.0, 0.0, 0.0, 0.125, 0.0, 0.0,
                1.0,
            ]),
            map::Dir::YzNeg => Mat4::from_cols_array(&[
                0.0, -0.0, 0.125, 0.0, 0.0, 0.125, 0.0, 0.0, -0.125, 0.0, 0.0, 0.0, -0.125, 0.0,
                0.0, 1.0,
            ]),
            map::Dir::ZxPos => Mat4::from_cols_array(&[
                -0.125, 0.0, 0.0, 0.0, 0.0, 0.0, 0.125, 0.0, 0.0, 0.125, 0.0, 0.0, 0.0, 0.125, 0.0,
                1.0,
            ]),
            map::Dir::ZxNeg => Mat4::from_cols_array(&[
                -0.125, -0.0, 0.0, 0.0, 0.0, 0.0, -0.125, 0.0, 0.0, -0.125, 0.0, 0.0, 0.0, -0.125,
                0.0, 1.0,
            ]),
        };
        // println!("spawn");

        meshes_tmp.push((
            quad_mesh(),
            Mat4::from_translation(point.into_vec3() * 0.25) * plane_trans,
        ));
    }

    let mut plane_entities = std::collections::HashMap::<usize, Entity>::new();
    for (ent, plane) in &mut query.iter() {
        plane_entities.insert(plane.buf_index, ent);
    }
    // let material = materials.add(MyMaterial {});
    let mut num_planes = 0;
    let mut spawn_mesh = {
        // FIXME: why is type inference for 'planes' broken?
        |position: VertexAttributeValues,
         color: VertexAttributeValues,
         index: Indices,
         planes: Vec<u32>| {
            let mut mesh = Mesh::new(bevy::render::pipeline::PrimitiveTopology::TriangleList); //  {

            info!(
                "spawn mesh {:?} {}",
                position.len(),
                // normal.len(),
                // uv.len(),
                match &index {
                    Indices::U16(x) => x.len(),
                    Indices::U32(x) => x.len(),
                }
            );

            mesh.set_attribute(bevy::render::mesh::Mesh::ATTRIBUTE_POSITION, position);
            // mesh.set_attribute(bevy::render::mesh::Mesh::ATTRIBUTE_NORMAL, color.clone()); // HACK
            // mesh.set_attribute(bevy::render::mesh::Mesh::ATTRIBUTE_UV_0, color.clone()); // HACK
            mesh.set_attribute(ATTRIBUTE_COLOR, color);
            mesh.set_indices(Some(index));

            let mesh_handle = meshes.add(mesh);
            commands
                .spawn_bundle(MeshBundle {
                    mesh: mesh_handle.clone(),
                    render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
                        pipeline_handle.clone(),
                    )]),
                    transform: Transform::from_xyz(0.0, 0.0, 0.0),
                    ..Default::default()
                })
                // .insert(material.clone())
                .insert(QuadRenderMesh);

            for p in planes.iter() {
                // glue local Plane component (ToDo: rename) to pre-existing 'plane' entities
                commands
                    .entity(
                        *plane_entities
                            .get(&num_planes)
                            .expect("missing entity for plane index"),
                    )
                    .insert_bundle(PlaneComponents {
                        plane: Plane {
                            mesh_handle: mesh_handle.clone(),
                            indices: [p + 0, p + 1, p + 2, p + 3],
                        },
                    });
                num_planes += 1;
            }
        }
    };

    let mut position = Vec::new();
    let mut color = Vec::new();
    let mut index = Vec::new();
    let mut planes = Vec::new();
    for (mesh, trans) in meshes_tmp.iter() {
        if position.len() > 256 * 256 - 4 {
            spawn_mesh(
                bevy::render::mesh::VertexAttributeValues::Float32x3(position),
                bevy::render::mesh::VertexAttributeValues::Float32x3(color),
                bevy::render::mesh::Indices::U32(index),
                planes,
            );

            position = Vec::new();
            color = Vec::new();
            index = Vec::new();
            planes = Vec::new();
        }

        let index_offset = position.len() as u32;
        planes.push(index_offset); // this is also the first index that belongs to the current plane
        match mesh.attribute(bevy::render::mesh::Mesh::ATTRIBUTE_POSITION) {
            Some(VertexAttributeValues::Float32x3(vs)) => {
                for v in vs {
                    let v: Vec3 = v.clone().into();
                    let v: Vec3 = (*trans * v.extend(1.0)).truncate().into();
                    position.push([v.x, v.y, v.z]);
                }
            }
            _ => panic!("expected Vertex_Position to be Float3"),
        };
        match mesh.attribute(ATTRIBUTE_COLOR) {
            Some(VertexAttributeValues::Float32x3(vs)) => color.append(&mut vs.clone()),
            _ => panic!("expected Vertex_Color to be Float3"),
        };
        // match mesh.attribute(bevy::render::mesh::Mesh::ATTRIBUTE_UV_0) {
        //     Some(VertexAttributeValues::Float2(vs)) => uv.append(&mut vs.clone()),
        //     _ => panic!("expected Vertex_Uv to be Float2"),
        // };

        match mesh.indices() {
            Some(bevy::render::mesh::Indices::U32(indices)) => {
                index.append(&mut indices.iter().map(|i| i + index_offset).collect());
            }
            _ => panic!("expected U32 index array"),
        }
    }
    spawn_mesh(
        bevy::render::mesh::VertexAttributeValues::Float32x3(position),
        bevy::render::mesh::VertexAttributeValues::Float32x3(color),
        bevy::render::mesh::Indices::U32(index),
        planes,
    );

    commands
        .spawn_bundle(PerspectiveCameraBundle {
            transform: Transform::from_matrix(Mat4::face_toward(
                Vec3::new(10.0, 5.0, 40.0),
                Vec3::new(10.0, 5.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            )),
            ..Default::default()
        })
        .insert(bevy_fly_camera::FlyCamera {
            mouse_drag: true,
            sensitivity: 8.0,
            ..Default::default()
        });
}
#[derive(Default)]
pub struct RadFrontbufState {
    pub updated: bool,
}
fn apply_frontbuf(
    front_buf: Res<rad::data::FrontBuf>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut fb_state: ResMut<RadFrontbufState>,
    query: Query<(&rad::PlaneIndex, &Plane)>,
) {
    if !fb_state.updated {
        return;
    }
    // {
    //     let buf = front_buf.read();
    //     info!("apply_frontbuf {} {} {}", buf.r[0], buf.g[0], buf.b[0]);
    // }
    fb_state.updated = false;
    let mut mesh_handle = Handle::<Mesh>::default();
    let mut new_vs = Vec::new();
    for (rad_plane, plane) in &mut query.iter() {
        // read rgb value from rad frontbuffer
        let buf = front_buf.read();
        let r = buf.r[rad_plane.buf_index];
        let g = buf.g[rad_plane.buf_index];
        let b = buf.b[rad_plane.buf_index];

        // this branch should be hit only once per mesh because consecutive planes
        // will normally be located in the same mesh
        if mesh_handle != plane.mesh_handle {
            // apply updated new_vs to current mesh (if Some)
            if let Some(mesh) = meshes.get_mut(&mesh_handle) {
                // println!("set attribute");
                mesh.set_attribute(ATTRIBUTE_COLOR, VertexAttributeValues::Float32x3(new_vs));
            }
            // get next mesh and init new_vs
            mesh_handle = plane.mesh_handle.clone();
            new_vs = match meshes
                .get(&mesh_handle)
                .expect("missing mesh referenced by plane")
                .attribute(ATTRIBUTE_COLOR)
            {
                // allocate new_vs using size of existing attribute-array (OPT-REMARK: assuming this is more efficient than cloning it... in-place update would be nice)
                Some(VertexAttributeValues::Float32x3(vs)) => vec![[0f32, 0f32, 0f32]; vs.len()],
                _ => panic!("expected Vertex_Normal to be Float3"),
            };
        }

        for i in plane.indices.iter() {
            new_vs[*i as usize][0] = r;
            new_vs[*i as usize][1] = g;
            new_vs[*i as usize][2] = b;
        }
    }
    // apply final updated new_vs to last mesh_opt (if Some)
    if let Some(mesh) = meshes.get_mut(&mesh_handle) {
        // println!("set attribute");
        mesh.set_attribute(ATTRIBUTE_COLOR, VertexAttributeValues::Float32x3(new_vs));
    }
}

fn quad_mesh() -> Mesh {
    let mut mesh = Mesh::from(shape::Quad {
        size: Vec2::new(2.0, 2.0),
        flip: false,
    });
    mesh.set_attribute(
        ATTRIBUTE_COLOR,
        VertexAttributeValues::from(vec![
            [0.0, 0.0, 0.5],
            [0.0, 0.0, 0.5],
            [0.0, 0.0, 0.5],
            [0.0, 0.0, 0.5],
        ]),
    );
    mesh
}

#[derive(Default)]
pub struct QuadRenderPlugin;

impl Plugin for QuadRenderPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_startup_system_to_stage("renderer", setup.system())
            // .add_system(blink_system.system())
            .add_system(apply_frontbuf.system())
            // .add_asset::<MyMaterial>()
            .add_asset::<StandardMaterial>()
            .init_resource::<RadFrontbufState>();
    }
}
