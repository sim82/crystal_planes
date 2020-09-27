use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, PrintDiagnosticsPlugin},
    input::mouse::MouseMotion,
    prelude::*,
    render::{
        mesh::{shape, VertexAttributeValues},
        pipeline::{DynamicBinding, PipelineDescriptor, PipelineSpecialization, RenderPipeline},
        render_graph::{base, AssetRenderResourcesNode, RenderGraph},
        renderer::RenderResources,
        shader::{ShaderStage, ShaderStages},
    },
};
mod crystal;
use rand::{thread_rng, Rng};
/// This example illustrates how to create a custom material asset and a shader that uses that material
fn main() {
    App::build()
        .add_default_plugins()
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(PrintDiagnosticsPlugin::default())
        .add_plugin(FlyCameraPlugin)
        .add_asset::<MyMaterial>()
        .add_startup_system(setup.system())
        .add_system(blink_system.system())
        .run();
}

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
    mesh_handle: Handle<Mesh>,
    indices: [u32; 4],
}

fn setup(
    mut commands: Commands,
    mut pipelines: ResMut<Assets<PipelineDescriptor>>,
    mut shaders: ResMut<Assets<Shader>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<MyMaterial>>,
    mut render_graph: ResMut<RenderGraph>,
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
        .with(FlyCamera::default());

    let bm = crystal::read_map("assets/maps/hidden_ramp.txt").expect("could not read file");
    let mut planes = crystal::PlanesSep::new();
    planes.create_planes(&*bm);

    let mut meshes_tmp = Vec::new();
    for p in planes.planes_iter() {
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
        println!("spawn");

        meshes_tmp.push((
            Mesh::from(shape::Quad {
                size: Vec2::new(2.0, 2.0),
                flip: false,
            }),
            Mat4::from_translation(point.into_vec3() * 0.25) * plane_trans,
        ));
    }

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
                commands.spawn(PlaneComponents {
                    plane: Plane {
                        mesh_handle,
                        indices: [p + 0, p + 1, p + 2, p + 3],
                    },
                });
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

fn blink_system(mut meshes: ResMut<Assets<Mesh>>, plane: &Plane) {
    // println!("blink");
    let mut mesh = meshes
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

pub struct FlyCamera {
    /// The speed the FlyCamera moves at. Defaults to `1.0`
    pub speed: f32,
    /// The maximum speed the FlyCamera can move at. Defaults to `0.5`
    pub max_speed: f32,
    /// The sensitivity of the FlyCamera's motion based on mouse movement. Defaults to `3.0`
    pub sensitivity: f32,
    /// The amount of deceleration to apply to the camera's motion. Defaults to `1.0`
    pub friction: f32,
    /// The current pitch of the FlyCamera in degrees. This value is always up-to-date, enforced by [FlyCameraPlugin](struct.FlyCameraPlugin.html)
    pub pitch: f32,
    /// The current pitch of the FlyCamera in degrees. This value is always up-to-date, enforced by [FlyCameraPlugin](struct.FlyCameraPlugin.html)
    pub yaw: f32,
    /// The current velocity of the FlyCamera. This value is always up-to-date, enforced by [FlyCameraPlugin](struct.FlyCameraPlugin.html)
    pub velocity: Vec3,
    /// Key used to move forward. Defaults to `W`
    pub key_forward: KeyCode,
    /// Key used to move backward. Defaults to `S
    pub key_backward: KeyCode,
    /// Key used to move left. Defaults to `A`
    pub key_left: KeyCode,
    /// Key used to move right. Defaults to `D`
    pub key_right: KeyCode,
    /// Key used to move up. Defaults to `Space`
    pub key_up: KeyCode,
    /// Key used to move forward. Defaults to `LShift`
    pub key_down: KeyCode,
}
impl Default for FlyCamera {
    fn default() -> Self {
        Self {
            speed: 1.0,
            max_speed: 0.5,
            sensitivity: 3.0,
            friction: 1.0,
            pitch: 0.0,
            yaw: 0.0,
            velocity: Vec3::zero(),
            key_forward: KeyCode::W,
            key_backward: KeyCode::S,
            key_left: KeyCode::A,
            key_right: KeyCode::D,
            key_up: KeyCode::Space,
            key_down: KeyCode::LShift,
        }
    }
}

fn forward_vector(rotation: &Quat) -> Vec3 {
    rotation.mul_vec3(Vec3::unit_z()).normalize()
}

fn forward_walk_vector(rotation: &Quat) -> Vec3 {
    let f = forward_vector(rotation);
    let f_flattened = Vec3::new(f.x(), 0.0, f.z()).normalize();
    f_flattened
}

fn strafe_vector(rotation: &Quat) -> Vec3 {
    // Rotate it 90 degrees to get the strafe direction
    Quat::from_rotation_y(90.0f32.to_radians())
        .mul_vec3(forward_walk_vector(rotation))
        .normalize()
}

fn movement_axis(input: &Res<Input<KeyCode>>, plus: KeyCode, minus: KeyCode) -> f32 {
    let mut axis = 0.0;
    if input.pressed(plus) {
        axis += 1.0;
    }
    if input.pressed(minus) {
        axis -= 1.0;
    }
    axis
}

fn camera_movement_system(
    time: Res<Time>,
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<(&mut FlyCamera, &mut Transform)>,
) {
    for (mut options, mut transform) in &mut query.iter() {
        let axis_h = movement_axis(&keyboard_input, options.key_right, options.key_left);
        let axis_v = movement_axis(&keyboard_input, options.key_backward, options.key_forward);

        let axis_float = movement_axis(&keyboard_input, options.key_up, options.key_down);

        let any_button_down = axis_h != 0.0 || axis_v != 0.0 || axis_float != 0.0;

        let rotation = transform.rotation();
        let accel: Vec3 = ((strafe_vector(&rotation) * axis_h)
            + (forward_walk_vector(&rotation) * axis_v)
            + (Vec3::unit_y() * axis_float))
            * options.speed;

        let friction: Vec3 = if options.velocity.length() != 0.0 && !any_button_down {
            options.velocity.normalize() * -1.0 * options.friction
        } else {
            Vec3::zero()
        };

        options.velocity += accel * time.delta_seconds;

        // clamp within max speed
        if options.velocity.length() > options.max_speed {
            options.velocity = options.velocity.normalize() * options.max_speed;
        }

        let delta_friction = friction * time.delta_seconds;

        options.velocity = if (options.velocity + delta_friction).sign() != options.velocity.sign()
        {
            Vec3::zero()
        } else {
            options.velocity + delta_friction
        };
        transform.translate(options.velocity);
        // *translation += options.velocity;
        // println!("cms: {:?} {:?}", *transform, options.velocity);
    }
}

#[derive(Default)]
struct State {
    mouse_motion_event_reader: EventReader<MouseMotion>,
}

fn mouse_motion_system(
    time: Res<Time>,
    mut state: ResMut<State>,
    mouse_motion_events: Res<Events<MouseMotion>>,
    mouse_button_input: Res<Input<MouseButton>>,
    mut query: Query<(&mut FlyCamera, &mut Transform)>,
) {
    if !mouse_button_input.pressed(MouseButton::Left) {
        return;
    }
    let mut delta: Vec2 = Vec2::zero();
    for event in state.mouse_motion_event_reader.iter(&mouse_motion_events) {
        delta += event.delta;
    }
    if delta == Vec2::zero() {
        return;
    }

    for (mut options, mut transform) in &mut query.iter() {
        options.yaw -= delta.x() * options.sensitivity * time.delta_seconds;
        options.pitch += delta.y() * options.sensitivity * time.delta_seconds;

        if options.pitch > 89.9 {
            options.pitch = 89.9;
        }
        if options.pitch < -89.9 {
            options.pitch = -89.9;
        }
        println!("pitch: {}, yaw: {}", options.pitch, options.yaw);

        let yaw_radians = options.yaw.to_radians();
        let pitch_radians = options.pitch.to_radians();

        transform.set_rotation(
            Quat::from_axis_angle(Vec3::unit_y(), yaw_radians)
                * Quat::from_axis_angle(-Vec3::unit_x(), pitch_radians),
        );
    }
}

pub struct FlyCameraPlugin;

impl Plugin for FlyCameraPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<State>()
            .add_system(camera_movement_system.system())
            .add_system(mouse_motion_system.system());
    }
}
