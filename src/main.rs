#![feature(slice_group_by)]

use std::sync::{
    mpsc::{self, Receiver, Sender},
    Mutex,
};

#[allow(unused_imports)]
use bevy::diagnostic::{Diagnostic, Diagnostics, DiagnosticsPlugin};

use bevy::{
    diagnostic::FrameTimeDiagnosticsPlugin, prelude::*, render::mesh::shape, winit::WinitConfig,
};
use bevy_egui::EguiPlugin;
use crystal_planes::{
    hud::{self, DemoSystemState, HudElement},
    hud_egui::{self, HudEguiPlugin, HudOrder},
    map,
    property::{self, PropertyName, PropertyRegistry, PropertyUpdateEvent, PropertyValue},
    quad_render, rad, util,
};

use rand::{thread_rng, Rng};

/// This example illustrates how to create a custom material asset and a shader that uses that material
fn main() {
    let planes_stage = SystemStage::single_threaded()
        .with_system(setup.system())
        .with_system(setup_bevy.system());

    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        // .add_plugin(PrintDiagnosticsPlugin::default())
        .add_plugin(bevy_fly_camera::FlyCameraPlugin)
        .add_plugin(property::PropertyPlugin)
        .add_startup_stage("planes", planes_stage)
        .add_startup_stage_after("planes", "renderer", SystemStage::single_threaded())
        .add_plugin(quad_render::QuadRenderPlugin::default())
        .add_system_to_stage(CoreStage::PostUpdate, light_update_system.system())
        .init_resource::<LightUpdateState>()
        .add_startup_system(setup_demo_system.system())
        .add_system(demo_system.system())
        .init_resource::<DemoSystemState>()
        // .add_plugin(hud::HudPlugin)
        .add_system(rotator_system.system())
        .init_resource::<RotatorSystemState>()
        .insert_resource(WinitConfig {
            return_from_run: true,
        })
        .add_system(rad_to_render_update.system())
        .add_startup_system(setup_diagnostic_system.system())
        .add_plugin(EguiPlugin)
        .add_plugin(HudEguiPlugin)
        .run();
    println!("run returned");
}

fn setup(mut commands: Commands) {
    let bm = map::read_map("assets/maps/hidden_ramp.txt").expect("could not read file");
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
    commands.insert_resource(front_buf);
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
    // property_registry: Res<PropertyRegistry>,
    property_registry: Res<PropertyRegistry>,
    mut query: Query<(&Rotator, &mut Transform)>,
    property_query: Query<(&PropertyName, &PropertyValue)>,
) {
    if let Some(ent) = property_registry.get("rotator_system.enabled") {
        if let Ok((_, PropertyValue::Bool(true))) = property_query.get(ent) {
            for (_rotator, mut transform) in query.iter_mut() {
                transform.rotate(Quat::from_rotation_y(0.5 * time.delta_seconds()));
            }
        }
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

fn rand_color(min: f32, max: f32) -> Vec3 {
    util::hsv_to_rgb(thread_rng().gen_range(min..max), 1f32, 1f32)
}
fn setup_demo_system(mut commands: Commands, mut hud_order: ResMut<HudOrder>) {
    let hud_group = "2. Demo System";
    commands
        .spawn()
        .insert(property::PropertyName("demo_system.light_enabled".into()))
        .insert(property::PropertyValue::Bool(true))
        .insert(HudElement::EditThis)
        .insert(hud_order.next().in_group(hud_group));
    commands
        .spawn()
        .insert(property::PropertyName("rotator_system.enabled".into()))
        .insert(property::PropertyValue::Bool(true))
        .insert(HudElement::EditThis)
        .insert(hud_order.next().in_group(hud_group));
    commands
        .spawn()
        .insert(property::PropertyName("demo_system.cycle".into()))
        .insert(property::PropertyValue::Bool(false))
        .insert(HudElement::EditThis)
        .insert(hud_order.next().in_group(hud_group));

    commands
        .spawn()
        .insert(property::PropertyName("demo_system.test_string".into()))
        .insert(property::PropertyValue::String("Hello Property!".into()))
        .insert(HudElement::EditThis)
        .insert(hud_order.next().in_group(hud_group));
    commands
        .spawn()
        .insert(property::PropertyName("demo_system.test_string2".into()))
        .insert(property::PropertyValue::String("Hello Property2!".into()))
        .insert(HudElement::EditThis)
        .insert(hud_order.next().in_group(hud_group));
}

fn demo_system(
    mut state: ResMut<DemoSystemState>,
    time: Res<Time>,
    property_registry: Res<PropertyRegistry>,
    rad_update_channel: Res<Mutex<Sender<rad::com::RenderToRad>>>,
    property_query: Query<(&PropertyName, &PropertyValue)>,
    property_query_changed: Query<(&PropertyName, &PropertyValue), Changed<PropertyValue>>,
) {
    state.cycle_timer.tick(time.delta());

    if state.cycle_timer.just_finished() {
        if let Some(ent) = property_registry.get("demo_system.cycle") {
            if let Ok((_, PropertyValue::Bool(true))) = property_query.get(ent) {
                rad_update_channel
                    .lock()
                    .unwrap()
                    .send(rad::com::RenderToRad::SetStripeColors(
                        rand_color(0f32, 180f32),
                        rand_color(180f32, 360f32),
                    ))
                    .unwrap();
            }
        }
    }
    if let Some(ent) = property_registry.get("demo_system.light_enabled") {
        if let Ok((_, PropertyValue::Bool(v))) = property_query_changed.get(ent) {
            rad_update_channel
                .lock()
                .unwrap()
                .send(rad::com::RenderToRad::EnablePointlights(*v))
                .unwrap();
        }
    }
    // if let Some(PropertyValue::Bool(v)) =
    //     state.light_enabled_tracker.get_changed(&property_registry)
    // {
    //     rad_update_channel
    //         .lock()
    //         .unwrap()
    //         .send(rad::com::RenderToRad::EnablePointlights(*v))
    //         .unwrap();
    // }
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
    mut fb_state: ResMut<quad_render::RadFrontbufState>,
    mut property_update_sender: EventWriter<PropertyUpdateEvent>,
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
                property_update_sender.send(PropertyUpdateEvent::new(
                    "rotator_system.enabled".to_string(),
                    PropertyValue::Bool(true),
                ))
            }
        }
    }
}
