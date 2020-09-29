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
        .add_startup_stage_after("planes", "renderer")
        .add_plugin(quad_render::QuadRenderPlugin::default())
        .add_system(light_move_system.system())
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

    let (send, recv) = mpsc::channel();
    let plane_scene = crystal::PlaneScene::new(planes, bm);
    let front_buf = rad::spawn_rad_update(extents, plane_scene.clone(), recv);

    commands
        .insert_resource(plane_scene)
        // .insert_resource(extents)
        .insert_resource(front_buf.clone())
        .insert_resource(Mutex::new(send))
        .spawn((PointLight::default(),));

    for i in 0..num_planes {
        commands.spawn(rad::PlaneBundle {
            plane: rad::Plane { buf_index: i },
        });
    }
}

// TODO: build from default components
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
    rad_update_channel: Res<Mutex<Sender<rad::RadUpdate>>>,
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
    println!("light move: {:?}", m);
    if m != Vec3::zero() {
        point_light.pos += m;
        rad_update_channel
            .lock()
            .unwrap()
            .send(rad::RadUpdate::PointLight(
                0,
                point_light.pos,
                point_light.color,
            ))
            .unwrap();
    }
}
