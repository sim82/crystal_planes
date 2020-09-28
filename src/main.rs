use std::sync;

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
mod fly_camera;
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
        .add_plugin(fly_camera::FlyCameraPlugin)
        .add_startup_stage("planes")
        .add_startup_system_to_stage("planes", setup.system())
        .add_startup_stage_after("planes", "renderer")
        .add_plugin(quad_render::QuadRenderPlugin::default())
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

    let front_buf = rad::spawn_rad_update(extents);
    // let front_buf = rad::FrontBuf::new(rad::RadBuffer::new_with(num_planes, 1.0, 0.5, 0.5));
    // let mut back_buf = rad::BackBuf(rad::RadBuffer::new_with(num_planes, 0.5, 0.5, 1.0));

    commands
        .insert_resource(crystal::PlaneScene {
            planes,
            blockmap: bm,
        })
        // .insert_resource(extents)
        .insert_resource(front_buf.clone());

    for i in 0..num_planes {
        commands.spawn(rad::PlaneBundle {
            plane: rad::Plane { buf_index: i },
        });
    }

    // std::thread::spawn(move || loop {
    //     {
    //         let mut front = front_buf.write();
    //         std::mem::swap(&mut *front, &mut back_buf.0);
    //     }
    //     std::thread::sleep(std::time::Duration::from_millis(200));
    // });
}

// fn swap_buffers(mut front: ResMut<rad::FrontBuf>, mut back: ResMut<rad::BackBuf>) {
//     std::mem::swap(front.0.get_mut().unwrap(), &mut back.0);
// }
