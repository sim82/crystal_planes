use bevy::prelude::*;
use crystal_planes::crystal::math::prelude::*;
use crystal_planes::octree;
use crystal_planes::octree_render;
fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_stage("renderer")
        .add_plugin(bevy_fly_camera::FlyCameraPlugin)
        .add_plugin(octree_render::OctreeRenderPlugin::default())
        .add_startup_system(setup.system())
        .run();
}

fn setup(
    mut commands: Commands,
    mut octants: ResMut<octree::Octants>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut vis_info: ResMut<octree_render::OctreeVisInfo>,
) {
    vis_info.show_level = Some(0);
    let points = [Point3i::new(0, 0, 0), Point3i::new(15, 15, 15)];
    vis_info.root = octree::create_octants_bottom_up(&mut *octants, &points);

    commands
        .spawn(Camera3dComponents {
            transform: Transform::new(Mat4::face_toward(
                Vec3::new(0.0, 0.0, 10.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            )),
            ..Default::default()
        })
        .with(bevy_fly_camera::FlyCamera {
            mouse_drag: true,
            sensitivity: 2.0,
            ..Default::default()
        })
        // light
        .spawn(LightComponents {
            transform: Transform::from_translation(Vec3::new(4.0, 5.0, -4.0)),
            ..Default::default()
        });
}
