use crate::octree::{self, util::OctreeLoad};
use std::collections::HashMap;
// use crate::octree::{self, util::OctreeLoad};
use bevy::{prelude::*, render::mesh::shape};
use rand::{thread_rng, Rng};

pub struct OctreeVisInfo {
    pub show_level: Option<u32>,
    cur_level: Option<u32>,
    cubes: HashMap<i32, Handle<Mesh>>,
    pub root: Option<octree::OctantId>,
    spawned: bool,
}

impl Default for OctreeVisInfo {
    fn default() -> OctreeVisInfo {
        OctreeVisInfo {
            show_level: None,
            cur_level: None,
            cubes: HashMap::new(),
            root: None,
            spawned: false,
        }
    }
}

fn setup(mut octants: ResMut<octree::Octants>, mut vis_info: ResMut<OctreeVisInfo>) {
    if vis_info.root.is_none() {
        vis_info.root = octants.load_map("assets/maps/hidden_ramp.txt");
        if !vis_info.root.is_some() {
            panic!("vis_info root not set and failed to load from map");
        }

        // .expect("failed to load octree from map");
    }
    // commands
    //     // light
    //     .spawn(LightComponents {
    //         transform: Transform::from_translation(Vec3::new(4.0, 5.0, -4.0)),
    //         ..Default::default()
    //     });
}

fn vis_update_system(
    mut commands: Commands,
    mut vis_info: ResMut<OctreeVisInfo>,
    octants: Res<octree::Octants>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut query: Query<(Mut<Draw>, &octree::OctantId, Entity)>,
    mut quad_query: Query<(Mut<Draw>, &super::quad_render::QuadRenderMesh)>,
) {
    let root = match vis_info.root {
        Some(root) => root,
        None => return,
    };

    match (vis_info.cur_level, vis_info.show_level) {
        (None, Some(level)) if !vis_info.spawned => {
            let height = octants.get(root).scale + 1;
            let mut num = 0;
            for id in octants.get_id_iter() {
                let octant = octants.get(id);
                let (pos, size) = octant.get_geometry(height);

                let cube_size = size.0 as f32 * 0.25 * 0.5;
                // let mesh = *vis_info
                //     .cubes
                //     .entry(size.0)
                //     .or_insert_with(|| meshes.add(Mesh::from(shape::Cube { size: cube_size })))
                //     .clone();

                let color = crate::crystal::util::hsv_to_rgb(
                    thread_rng().gen_range(0f32, 360f32),
                    1f32,
                    1f32,
                );
                let cube_material_handle = materials.add(StandardMaterial {
                    albedo: Color::rgba(color.x(), color.y(), color.z(), 1.0),
                    ..Default::default()
                });

                // commands
                //     .spawn(PbrComponents {
                //         mesh,
                //         material: cube_material_handle,
                //         transform: Transform::from_translation(
                //             pos.into_vec3() * 0.25 + Vec3::splat(cube_size - 0.125),
                //         ),
                //         draw: Draw {
                //             is_transparent: false,
                //             is_visible: level == octant.scale,
                //             ..Default::default()
                //         },
                //         ..Default::default()
                //     })
                //     .with(id);

                num += 1;
            }
            println!("spawned: {}", num);
            vis_info.spawned = true;

            for (mut draw, _) in quad_query.iter_mut() {
                draw.is_visible = false;
            }
        }
        (None, Some(level)) if vis_info.spawned => {
            for (mut draw, id, _) in query.iter_mut() {
                draw.is_visible = Some(level) == Some(octants.get(*id).scale);
                // println!("draw: {}", draw.is_visible);
            }
            for (mut draw, _) in quad_query.iter_mut() {
                draw.is_visible = false;
            }
        }
        (Some(old_level), Some(level)) if old_level != level => {
            for (mut draw, id, _) in query.iter_mut() {
                draw.is_visible = Some(level) == Some(octants.get(*id).scale);
                // println!("draw: {}", draw.is_visible);
            }
        }
        (Some(_), None) => {
            for (mut draw, _, _) in query.iter_mut() {
                draw.is_visible = false;
                // println!("draw: {}", draw.is_visible);
            }
            for (mut draw, _) in quad_query.iter_mut() {
                draw.is_visible = true;
            }
        }
        _ => (),
    }
    vis_info.cur_level = vis_info.show_level;
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
