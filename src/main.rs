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
use rand::{thread_rng, Rng};
/// This example illustrates how to create a custom material asset and a shader that uses that material
fn main() {
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

    App::build()
        .add_default_plugins()
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(PrintDiagnosticsPlugin::default())
        .add_plugin(fly_camera::FlyCameraPlugin)
        .add_resource(crystal::PlaneScene {
            planes,
            blockmap: bm,
        })
        .add_resource(extents)
        .add_startup_system(setup.system())
        .add_plugin(quad_render::QuadRenderPlugin::default())
        .run();
}

fn setup(mut commands: Commands) {}
