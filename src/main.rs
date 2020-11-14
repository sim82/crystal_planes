use std::sync::{
    mpsc::{self, Sender},
    Mutex,
};

#[allow(unused_imports)]
use bevy::diagnostic::PrintDiagnosticsPlugin;

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
    App::build()
        .add_plugins(DefaultPlugins)
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        // .add_plugin(PrintDiagnosticsPlugin::default())
        .add_plugin(bevy_fly_camera::FlyCameraPlugin)
        .add_startup_stage("planes")
        .add_startup_system_to_stage("planes", setup.system())
        .add_startup_system_to_stage("planes", setup_bevy.system())
        .add_startup_stage_after("planes", "renderer")
        .add_plugin(quad_render::QuadRenderPlugin::default())
        .add_plugin(octree_render::OctreeRenderPlugin::default())
        //.add_system(light_move_system.system())
        .add_system_to_stage(stage::POST_UPDATE, light_update_system.system())
        .init_resource::<LightUpdateState>()
        .add_system(demo_system.system())
        .init_resource::<DemoSystemState>()
        .add_plugin(hud::HudPlugin)
        .add_system(rotator_system.system())
        .init_resource::<RotatorSystemState>()
        .add_resource(WinitConfig {
            return_from_run: true,
        })
        // .add_system(swap_buffers.system())
        .run();

    println!("run returned");
}

fn setup(mut commands: Commands) {
    let bm = map::read_map("assets/maps/hidden_ramp.txt").expect("could not read file");
    let mut planes = map::PlanesSep::new();
    planes.create_planes(&*bm);

    let num_planes = planes.num_planes();

    let (render_to_rad_send, render_to_rad_recv) = mpsc::channel();
    let plane_scene = map::PlaneScene::new(planes, bm);
    let (front_buf, rad_to_render_recv) =
        rad::worker::spawn_rad_update(plane_scene.clone(), render_to_rad_recv);

    commands
        .insert_resource(plane_scene)
        // .insert_resource(extents)
        .insert_resource(front_buf.clone())
        .insert_resource(Mutex::new(render_to_rad_send))
        .insert_resource(Mutex::new(rad_to_render_recv))
        .spawn((PointLight::default(),));

    for i in 0..num_planes {
        commands.spawn((rad::PlaneIndex { buf_index: i },));
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
                })
                // light
                .spawn(LightComponents {
                    transform: Transform::from_translation(Vec3::new(0.0, 0.0, 25.0)),
                    ..Default::default()
                });
        })
        // camera
        .spawn(Camera3dComponents {
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
    rad_update_channel: Res<Mutex<Sender<rad::worker::RenderToRad>>>,
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
            .send(rad::worker::RenderToRad::PointLight(
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
    rad_update_channel: Res<Mutex<Sender<rad::worker::RenderToRad>>>,
    rad_light: &RadPointLight,
    transform: &GlobalTransform,
    // Mutated<GlobalTransform>)>,
    // _: Mutated<Position>,
) {
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
        .send(rad::worker::RenderToRad::PointLight(
            0,
            pos,
            rad_light.color,
        ))
        .unwrap();
}

use hud::DemoSystemState;
impl Default for DemoSystemState {
    fn default() -> Self {
        DemoSystemState {
            cycle: false,
            cycle_timer: Timer::from_seconds(1f32, true),
        }
    }
}

fn rand_color(min: f32, max: f32) -> Vec3 {
    util::hsv_to_rgb(thread_rng().gen_range(min, max), 1f32, 1f32)
}

fn demo_system(
    mut state: ResMut<DemoSystemState>,
    time: Res<Time>,
    rad_update_channel: Res<Mutex<Sender<rad::worker::RenderToRad>>>,
) {
    state.cycle_timer.tick(time.delta_seconds);
    if state.cycle && state.cycle_timer.just_finished {
        rad_update_channel
            .lock()
            .unwrap()
            .send(rad::worker::RenderToRad::SetStripeColors(
                rand_color(0f32, 180f32),
                rand_color(180f32, 360f32),
            ))
            .unwrap();
    }
}
