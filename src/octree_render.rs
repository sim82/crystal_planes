use bevy::{prelude::*, render::mesh::shape};
use crystal_planes::octree::{self, util::OctreeLoad};
use rand::{thread_rng, Rng};

#[derive(Default)]
pub struct OctreeVisInfo {
    pub show_level: Option<u32>,
    cur_level: Option<u32>,
}

fn setup(
    mut commands: Commands,
    mut octants: ResMut<octree::Octants>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let root = octants
        .load_map("assets/maps/hidden_ramp.txt")
        .expect("failed to load octree from map");

    let height = octants.get(root).level;

    let mut cubes = std::collections::HashMap::new();

    for id in octants.get_id_iter() {
        let octant = octants.get(id);
        let (pos, size) = octant.get_geometry(height);

        let cube_size = size.0 as f32 * 0.25 * 0.5;
        let mesh = *cubes
            .entry(size.0)
            .or_insert_with(|| meshes.add(Mesh::from(shape::Cube { size: cube_size })));

        let color = crystal_planes::crystal::util::hsv_to_rgb(
            thread_rng().gen_range(0f32, 360f32),
            1f32,
            1f32,
        );
        let cube_material_handle = materials.add(StandardMaterial {
            albedo: Color::rgba(color.x(), color.y(), color.z(), 1.0),
            ..Default::default()
        });

        commands
            .spawn(PbrComponents {
                mesh,
                material: cube_material_handle,
                transform: Transform::from_translation(
                    pos.into_vec3() * 0.25 + Vec3::splat(cube_size - 0.125),
                ),
                draw: Draw {
                    is_transparent: false,
                    is_visible: false,
                    ..Default::default()
                },
                ..Default::default()
            })
            .with(id);
    }

    commands
        // light
        .spawn(LightComponents {
            transform: Transform::from_translation(Vec3::new(4.0, 5.0, -4.0)),
            ..Default::default()
        });
}

fn vis_update_system(
    mut vis_info: ResMut<OctreeVisInfo>,
    octants: Res<octree::Octants>,
    mut query: Query<(Mut<Draw>, &octree::OctantId)>,
    mut quad_query: Query<(Mut<Draw>, &super::quad_render::QuadRenderMesh)>,
) {
    if vis_info.cur_level != vis_info.show_level {
        // println!(
        //     "vis info: {:?} {:?}",
        //     vis_info.cur_level, vis_info.show_level
        // );
        for (mut draw, id) in &mut query.iter() {
            draw.is_visible = vis_info.show_level == Some(octants.get(*id).level);
            // println!("draw: {}", draw.is_visible);
        }

        for (mut draw, _) in &mut quad_query.iter() {
            draw.is_visible = vis_info.show_level.is_none();
        }

        vis_info.cur_level = vis_info.show_level;
    }
}

#[derive(Default)]
pub struct OctreeRenderPlugin;

impl Plugin for OctreeRenderPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<octree::Octants>()
            .init_resource::<OctreeVisInfo>()
            .add_startup_system_to_stage("renderer", setup.system())
            .add_system(vis_update_system.system());
    }
}
