use std::sync::{
    self,
    mpsc::{self, Sender},
    Mutex,
};

use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, PrintDiagnosticsPlugin},
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
mod quad_render;
use crystal::ffs;
use crystal::rad;
use rand::{thread_rng, Rng};
/// This example illustrates how to create a custom material asset and a shader that uses that material
fn main() {
    App::build()
        .add_default_plugins()
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(PrintDiagnosticsPlugin::default())
        .add_plugin(bevy_fly_camera::FlyCameraPlugin)
        .add_startup_stage("planes")
        .add_startup_system_to_stage("planes", setup.system())
        .add_startup_system_to_stage("planes", setup_bevy.system())
        .add_startup_stage_after("planes", "renderer")
        .add_plugin(quad_render::QuadRenderPlugin::default())
        //.add_system(light_move_system.system())
        .add_system(light_update_system.system())
        .add_system(rotator_system.system())
        // .add_system(swap_buffers.system())
        .run();
}

fn setup(mut commands: Commands) {
    let bm = crystal::read_map("assets/maps/hidden_ramp.txt").expect("could not read file");
    let mut planes = crystal::PlanesSep::new();
    planes.create_planes(&*bm);

    let extents = match ffs::Extents::load("extents.bin") {
        Some(extents) => extents,
        None => {
            let formfactors = ffs::split_formfactors(ffs::setup_formfactors(&planes, &*bm));
            let extents = ffs::Extents(ffs::to_extents(&formfactors));
            extents.write("extents.bin");
            extents
        }
    };

    let num_planes = planes.num_planes();

    let (render_to_rad_send, render_to_rad_recv) = mpsc::channel();
    let plane_scene = crystal::PlaneScene::new(planes, bm);
    let (front_buf, rad_to_render_recv) =
        rad::spawn_rad_update(extents, plane_scene.clone(), render_to_rad_recv);

    commands
        .insert_resource(plane_scene)
        // .insert_resource(extents)
        .insert_resource(front_buf.clone())
        .insert_resource(Mutex::new(render_to_rad_send))
        .insert_resource(Mutex::new(rad_to_render_recv))
        .spawn((PointLight::default(),));

    for i in 0..num_planes {
        commands.spawn(rad::PlaneBundle {
            plane: rad::Plane { buf_index: i },
        });
    }
}

/// this component indicates what entities should rotate
struct Rotator;

/// rotates the parent, which will result in the child also rotating
fn rotator_system(time: Res<Time>, mut query: Query<(&Rotator, &mut Transform)>) {
    for (_rotator, mut transform) in &mut query.iter() {
        transform.rotate(Quat::from_rotation_y(0.5 * time.delta_seconds));
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
        albedo: Color::rgb(0.5, 0.4, 0.3),
        ..Default::default()
    });

    let sphere_handle = meshes.add(Mesh::from(shape::Icosphere::default()));
    let sphere_material_handle = materials.add(StandardMaterial {
        albedo: Color::rgb(1.0, 0.9, 0.8),
        ..Default::default()
    });

    commands
        // parent cube
        .spawn(PbrComponents {
            mesh: cube_handle,
            material: cube_material_handle,
            transform: Transform::from_translation(Vec3::new(0.0, 20.0, 0.0)),
            ..Default::default()
        })
        .with(Rotator)
        .with_children(|parent| {
            // child cube
            parent
                .spawn(PbrComponents {
                    mesh: sphere_handle,
                    material: sphere_material_handle,
                    transform: Transform::from_translation(Vec3::new(0.0, 0.0, 30.0)),
                    ..Default::default()
                })
                .with(RadPointLight {
                    color: Vec3::new(1.0, 0.9, 0.8),
                });
        })
        // light
        .spawn(LightComponents {
            transform: Transform::from_translation(Vec3::new(4.0, 5.0, -4.0)),
            ..Default::default()
        })
        // camera
        .spawn(Camera3dComponents {
            transform: Transform::new(Mat4::face_toward(
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

fn light_move_system(
    keyboard_input: Res<Input<KeyCode>>,
    rad_update_channel: Res<Mutex<Sender<rad::RenderToRad>>>,
    mut point_light: Mut<PointLight>,
) {
    let mut m = Vec3::zero();
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
    if m != Vec3::zero() {
        point_light.pos += m;
        rad_update_channel
            .lock()
            .unwrap()
            .send(rad::RenderToRad::PointLight(
                0,
                point_light.pos,
                point_light.color,
            ))
            .unwrap();
    }
}

fn light_update_system(
    rad_update_channel: Res<Mutex<Sender<rad::RenderToRad>>>,
    rad_light: &RadPointLight,
    transform: &GlobalTransform,
) {
    rad_update_channel
        .lock()
        .unwrap()
        .send(rad::RenderToRad::PointLight(
            0,
            transform.translation() * 4.0,
            rad_light.color,
        ))
        .unwrap();
}
