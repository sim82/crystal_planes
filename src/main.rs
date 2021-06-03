use std::sync::{
    mpsc::{self, Receiver, Sender},
    Mutex,
};

#[allow(unused_imports)]
use bevy::diagnostic::{Diagnostic, Diagnostics, DiagnosticsPlugin};

use bevy::{
    diagnostic::FrameTimeDiagnosticsPlugin, prelude::*, render::mesh::shape, winit::WinitConfig,
};
use rand::{thread_rng, Rng};
mod hud;
mod map;
mod math;
mod octree;
mod octree_render;
mod quad_render;
mod rad;
mod util;

/// This example illustrates how to create a custom material asset and a shader that uses that material
fn main() {
    // if !false {
    let planes_stage = SystemStage::single_threaded()
        .with_system(setup.system())
        .with_system(setup_bevy.system());
    App::build()
        // .add_stage_after(stage::UPDATE, "background", SystemStage::parallel())
        .add_plugins(DefaultPlugins)
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        // .add_plugin(PrintDiagnosticsPlugin::default())
        .add_plugin(bevy_fly_camera::FlyCameraPlugin)
        .add_startup_stage("planes", planes_stage)
        .add_startup_stage_after("planes", "renderer", SystemStage::single_threaded())
        .add_plugin(quad_render::QuadRenderPlugin::default())
        //.add_plugin(octree_render::OctreeRenderPlugin::default())
        //.add_system(light_move_system.system())
        .add_system_to_stage(CoreStage::PostUpdate, light_update_system.system())
        .init_resource::<LightUpdateState>()
        .add_system(demo_system.system())
        .init_resource::<DemoSystemState>()
        .add_plugin(hud::HudPlugin)
        .add_system(rotator_system.system())
        .init_resource::<RotatorSystemState>()
        .insert_resource(WinitConfig {
            return_from_run: true,
        })
        .add_system(rad_to_render_update.system())
        .add_startup_system(setup_diagnostic_system.system())
        // .add_startup_system(custom_attribute::setup.system())
        // .add_system_to_stage("background", test_background.system())
        // .add_system_to_stage("background", test_background2.system())
        // .add_plugin(mesh_custom_attribute::TestPlugin)
        // .add_system(swap_buffers.system())
        .run();
    // } else {
    //     App::build()
    //         .add_plugins(DefaultPlugins)
    //         .add_startup_system(custom_attribute::setup.system())
    //         .run();
    // }
    println!("run returned");
}

fn setup(mut commands: Commands) {
    let bm = map::read_map("projects/crystal_planes/assets/maps/hidden_ramp.txt")
        .expect("could not read file");
    let bm = Box::new(map::DenseBlockmap::from_bitmap(&*bm));
    let mut planes = map::PlanesSep::new();
    planes.create_planes(&*bm);

    let num_planes = planes.num_planes();

    let (render_to_rad_send, render_to_rad_recv) = mpsc::channel();
    let plane_scene = map::PlaneScene::new(planes, bm);
    let (front_buf, rad_to_render_recv) =
        rad::worker::spawn_rad_update(plane_scene.clone(), render_to_rad_recv);

    commands.insert_resource(plane_scene);
    // .insert_resource(extents)
    commands.insert_resource(front_buf.clone());
    commands.insert_resource(Mutex::new(render_to_rad_send));
    commands.insert_resource(Mutex::new(rad_to_render_recv));
    commands.spawn().insert(PointLight::default());

    for i in 0..num_planes {
        commands.spawn().insert(rad::PlaneIndex { buf_index: i });
    }
}

/// this component indicates what entities should rotate
struct Rotator;

use quad_render::RotatorSystemState;

/// rotates the parent, which will result in the child also rotating
fn rotator_system(
    time: Res<Time>,
    state: Res<RotatorSystemState>,
    mut query: Query<(&Rotator, &mut Transform)>,
) {
    if !state.run {
        return;
    }
    for (_rotator, mut transform) in query.iter_mut() {
        transform.rotate(Quat::from_rotation_y(0.5 * time.delta_seconds()));
    }
}

// stupid name for a system...
fn setup_bevy(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let cube_handle = meshes.add(Mesh::from(shape::Cube { size: 1.0 }));
    let cube_material_handle = materials.add(StandardMaterial {
        base_color: Color::rgb(0.5, 0.4, 0.3),
        ..Default::default()
    });

    let sphere_handle = meshes.add(Mesh::from(shape::Icosphere::default()));
    let sphere_material_handle = materials.add(StandardMaterial {
        base_color: Color::rgb(1.0, 0.9, 0.8),
        ..Default::default()
    });

    commands
        // parent cube
        .spawn_bundle(PbrBundle {
            mesh: cube_handle,
            material: cube_material_handle,
            transform: Transform::from_translation(Vec3::new(0.0, 20.0, 0.0)),
            ..Default::default()
        })
        .insert(Rotator)
        .with_children(|parent| {
            // child cube
            parent
                .spawn_bundle(PbrBundle {
                    mesh: sphere_handle,
                    material: sphere_material_handle,
                    transform: Transform::from_translation(Vec3::new(0.0, 0.0, 30.0)),
                    ..Default::default()
                })
                .insert(RadPointLight {
                    color: Vec3::new(1.0, 0.9, 0.8),
                });
            // light
            parent.spawn_bundle(PointLightBundle {
                transform: Transform::from_translation(Vec3::new(0.0, 0.0, 25.0)),
                ..Default::default()
            });
        });
    // camera
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_matrix(Mat4::face_toward(
            Vec3::new(5.0, 10.0, 10.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        )),
        ..Default::default()
    });
}
// TODO: build from default components
struct RadPointLight {
    color: Vec3,
}

#[allow(dead_code)]
struct PointLight {
    pos: Vec3,
    color: Vec3,
}

impl Default for PointLight {
    fn default() -> Self {
        PointLight {
            pos: Vec3::new(40f32, 20f32, 40f32),
            color: Vec3::new(1.0, 0.9, 0.8),
        }
    }
}

fn _light_move_system(
    keyboard_input: Res<Input<KeyCode>>,
    rad_update_channel: Res<Mutex<Sender<rad::com::RenderToRad>>>,
    mut point_light: Mut<PointLight>,
) {
    let mut m = Vec3::ZERO;
    if keyboard_input.pressed(KeyCode::Left) {
        m += Vec3::new(-1f32, 0f32, 0f32);
    }
    if keyboard_input.pressed(KeyCode::Right) {
        m += Vec3::new(1f32, 0f32, 0f32);
    }
    if keyboard_input.pressed(KeyCode::Up) {
        m += Vec3::new(0f32, 0f32, -1f32);
    }
    if keyboard_input.pressed(KeyCode::Down) {
        m += Vec3::new(0f32, 0f32, 1f32);
    }
    // println!("light move: {:?}", m);
    if m != Vec3::ZERO {
        point_light.pos += m;
        rad_update_channel
            .lock()
            .unwrap()
            .send(rad::com::RenderToRad::PointLight(
                0,
                point_light.pos,
                point_light.color,
            ))
            .unwrap();
    }
}

#[derive(Default)]
struct LightUpdateState {
    // pause: bool,
    last_pos: Option<Vec3>,
}

fn light_update_system(
    mut state: ResMut<LightUpdateState>,
    rad_update_channel: Res<Mutex<Sender<rad::com::RenderToRad>>>,
    query: Query<(&RadPointLight, &GlobalTransform)>, // Mutated<GlobalTransform>)>,
                                                      // _: Mutated<Position>
) {
    for (rad_light, transform) in query.iter() {
        let pos = transform.translation * 4.0;

        // FIXME: shouldn't Mutated<GlobalTransform>)> do this?
        if Some(pos) == state.last_pos {
            return;
        }
        // println!("send: {:?}", pos);

        state.last_pos = Some(pos);

        rad_update_channel
            .lock()
            .unwrap()
            .send(rad::com::RenderToRad::PointLight(0, pos, rad_light.color))
            .unwrap();
    }
}

use hud::DemoSystemState;
impl Default for DemoSystemState {
    fn default() -> Self {
        DemoSystemState {
            cycle: false,
            cycle_timer: Timer::from_seconds(1f32, true),
            light_enabled: true,
            light_enabled_target: true,
        }
    }
}

fn rand_color(min: f32, max: f32) -> Vec3 {
    util::hsv_to_rgb(thread_rng().gen_range(min, max), 1f32, 1f32)
}

fn demo_system(
    mut state: ResMut<DemoSystemState>,
    time: Res<Time>,
    rad_update_channel: Res<Mutex<Sender<rad::com::RenderToRad>>>,
) {
    state.cycle_timer.tick(time.delta());
    if state.cycle && state.cycle_timer.just_finished() {
        rad_update_channel
            .lock()
            .unwrap()
            .send(rad::com::RenderToRad::SetStripeColors(
                rand_color(0f32, 180f32),
                rand_color(180f32, 360f32),
            ))
            .unwrap();
    }

    if state.light_enabled != state.light_enabled_target {
        state.light_enabled = state.light_enabled_target;
        rad_update_channel
            .lock()
            .unwrap()
            .send(rad::com::RenderToRad::EnablePointlights(
                state.light_enabled,
            ))
            .unwrap();
    }
}

fn setup_diagnostic_system(mut diagnostics: ResMut<Diagnostics>) {
    // Diagnostics must be initialized before measurements can be added.
    // In general it's a good idea to set them up in a "startup system".
    diagnostics.add(Diagnostic::new(
        hud::RAD_INT_PER_SECOND,
        "rad_int_per_second",
        10,
    ));
}

fn rad_to_render_update(
    rad_to_render: Res<Mutex<Receiver<rad::com::RadToRender>>>,
    mut diagnostics: ResMut<Diagnostics>,
    mut render_status: ResMut<crate::hud::RenderStatus>,
    mut rotator_system_state: ResMut<RotatorSystemState>,
    mut fb_state: ResMut<quad_render::RadFrontbufState>,
) {
    for cmd in rad_to_render.lock().unwrap().try_iter() {
        match cmd {
            rad::com::RadToRender::IterationDone { num_int, duration } => {
                diagnostics.add_measurement(
                    hud::RAD_INT_PER_SECOND,
                    num_int as f64 / duration.as_secs_f64(),
                );
                fb_state.updated = true;
            }
            rad::com::RadToRender::StatusUpdate(text) => {
                render_status.text = text;
            }
            rad::com::RadToRender::RadReady => {
                render_status.text = "ready".into();
                rotator_system_state.run = true;
            }
        }
    }
}

// mod custom_attribute {

//     use bevy::{
//         prelude::*,
//         render::{
//             mesh::VertexAttributeValues,
//             pipeline::{PipelineDescriptor, RenderPipeline},
//             shader::{ShaderStage, ShaderStages},
//         },
//     };

//     pub fn setup(
//         mut commands: Commands,
//         mut pipelines: ResMut<Assets<PipelineDescriptor>>,
//         mut shaders: ResMut<Assets<Shader>>,
//         mut meshes: ResMut<Assets<Mesh>>,
//     ) {
//         const VERTEX_SHADER: &str = r#"
//         #version 450
//         layout(location = 0) in vec3 Vertex_Position;
//         layout(location = 1) in vec3 Vertex_Color;
//         layout(location = 0) out vec3 v_color;

//         layout(set = 0, binding = 0) uniform CameraViewProj {
//             mat4 ViewProj;
//         };
//         layout(set = 1, binding = 0) uniform Transform {
//             mat4 Model;
//         };
//         void main() {
//             gl_Position = ViewProj * Model * vec4(Vertex_Position, 1.0);
//             v_color = Vertex_Color;
//         }
//         "#;

//         const FRAGMENT_SHADER: &str = r#"
//         #version 450
//         layout(location = 0) out vec4 o_Target;
//         layout(location = 0) in vec3 v_color;

//         void main() {
//             o_Target = vec4(v_color, 1.0);
//         }
//         "#;

//         // Create a new shader pipeline
//         let pipeline_handle = pipelines.add(PipelineDescriptor::default_config(ShaderStages {
//             vertex: shaders.add(Shader::from_glsl(ShaderStage::Vertex, VERTEX_SHADER)),
//             fragment: Some(shaders.add(Shader::from_glsl(ShaderStage::Fragment, FRAGMENT_SHADER))),
//         }));

//         // create a generic cube
//         let mut cube_with_vertex_colors = Mesh::from(shape::Cube { size: 2.0 });

//         // insert our custom color attribute with some nice colors!
//         cube_with_vertex_colors.set_attribute(
//             // name of the attribute
//             "Vertex_Color",
//             // the vertex attributes, represented by `VertexAttributeValues`
//             // NOTE: the attribute count has to be consistent across all attributes, otherwise bevy
//             // will panic.
//             VertexAttributeValues::from(vec![
//                 // top
//                 [0.79, 0.73, 0.07],
//                 [0.74, 0.14, 0.29],
//                 [0.08, 0.55, 0.74],
//                 [0.20, 0.27, 0.29],
//                 // bottom
//                 [0.79, 0.73, 0.07],
//                 [0.74, 0.14, 0.29],
//                 [0.08, 0.55, 0.74],
//                 [0.20, 0.27, 0.29],
//                 // right
//                 [0.79, 0.73, 0.07],
//                 [0.74, 0.14, 0.29],
//                 [0.08, 0.55, 0.74],
//                 [0.20, 0.27, 0.29],
//                 // left
//                 [0.79, 0.73, 0.07],
//                 [0.74, 0.14, 0.29],
//                 [0.08, 0.55, 0.74],
//                 [0.20, 0.27, 0.29],
//                 // front
//                 [0.79, 0.73, 0.07],
//                 [0.74, 0.14, 0.29],
//                 [0.08, 0.55, 0.74],
//                 [0.20, 0.27, 0.29],
//                 // back
//                 [0.79, 0.73, 0.07],
//                 [0.74, 0.14, 0.29],
//                 [0.08, 0.55, 0.74],
//                 [0.20, 0.27, 0.29],
//             ]),
//         );
//         // cube
//         commands.spawn_bundle(MeshBundle {
//             mesh: meshes.add(cube_with_vertex_colors), // use our cube with vertex colors
//             render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
//                 pipeline_handle,
//             )]),
//             transform: Transform::from_xyz(0.0, 0.0, 0.0),
//             ..Default::default()
//         });
//         // // camera
//         // commands.spawn_bundle(PerspectiveCameraBundle {
//         //     transform: Transform::from_xyz(3.0, 5.0, -8.0).looking_at(Vec3::ZERO, Vec3::Y),
//         //     ..Default::default()
//         // });
//     }
// }
// fn test_background() {
//     info!("background");
//     // std::thread::sleep(std::time::Duration::from_secs(1))
// }

// fn test_background2() {
//     info!("background2");
// }
