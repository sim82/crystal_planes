use bevy::{prelude::*, render::mesh::shape};
use crystal_planes::octree::{self, util::OctreeLoad};
use rand::{thread_rng, Rng};

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup.system())
        .add_plugin(bevy_fly_camera::FlyCameraPlugin)
        .init_resource::<octree::Octants>()
        // .add_system(camera_order_color_system.system())
        .run();
}

struct OctreeLevel {
    pub level: u32,
}

fn setup(
    commands: &mut Commands,
    mut octants: ResMut<octree::Octants>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let root = octants
        .load_map("assets/maps/hidden_ramp.txt")
        .expect("failed to load octree from map");

    let height = octants.get(root).scale;
    let _max_level = (height + 1) as i32;
    // let cube_meshes = (0..max_level).map(|level| {
    //     meshes.add(Mesh::from(shape::Cube {
    //         size: (2.0f32.powi(level)) as f32,
    //     }))
    // });

    let mut cubes = std::collections::HashMap::new();

    for octant in octants.octants.iter() {
        // if octant.level != 0 {
        //     continue;
        // }
        let (pos, size) = octant.get_geometry(height);
        let mesh = cubes
            .entry(size.x)
            .or_insert_with(|| {
                meshes
                    .add(Mesh::from(shape::Cube {
                        size: size.x as f32 * 0.125 * 0.5,
                    }))
                    .clone()
            })
            .clone();

        let color =
            crystal_planes::util::hsv_to_rgb(thread_rng().gen_range(0f32, 360f32), 1f32, 1f32);
        let cube_material_handle = materials.add(StandardMaterial {
            albedo: Color::rgba(color.x, color.y, color.z, 1.0),
            ..Default::default()
        });

        commands
            .spawn(PbrBundle {
                mesh,
                material: cube_material_handle,
                transform: Transform::from_translation(pos.into_vec3() * 0.125),
                visible: Visible {
                    is_transparent: false,
                    is_visible: octant.scale == 2,
                    ..Default::default()
                },
                ..Default::default()
            })
            .with((OctreeLevel {
                level: octant.scale,
            },));
    }
    // commands
    //     // parent cube
    //     .spawn(PbrComponents {
    //         mesh: cube_handle,
    //         material: cube_material_handle,
    //         transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
    //         draw: Draw {
    //             is_transparent: true,
    //             ..Default::default()
    //         },
    //         ..Default::default()
    //     })
    //     .spawn(PbrComponents {
    //         mesh: cube_handle2,
    //         material: cube_material_handle,
    //         transform: Transform::from_translation(Vec3::new(0.1, 0.1, 0.1)),
    //         draw: Draw {
    //             is_transparent: true,
    //             ..Default::default()
    //         },
    //         ..Default::default()
    //     })
    //     .spawn(PbrComponents {
    //         mesh: cube_handle3,
    //         material: cube_material_handle,
    //         transform: Transform::from_translation(Vec3::new(0.11, 0.11, 0.11)),
    //         draw: Draw {
    //             is_transparent: true,
    //             ..Default::default()
    //         },
    //         ..Default::default()
    //     })

    commands
        .spawn(Camera3dBundle {
            transform: Transform::from_matrix(Mat4::face_toward(
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
        .spawn(LightBundle {
            transform: Transform::from_translation(Vec3::new(4.0, 5.0, -4.0)),
            ..Default::default()
        });
}

// fn camera_order_color_system(
//     mut materials: ResMut<Assets<StandardMaterial>>,
//     mut camera_query: Query<(&Camera, &VisibleEntities)>,
//     material_query: Query<&Handle<StandardMaterial>>,
// ) {
//     for (_camera, visible_entities) in &mut camera_query.iter() {
//         for visible_entity in visible_entities.iter() {
//             if let Ok(material_handle) = material_query.get(visible_entity.entity) {
//                 let material = materials.get_mut(material_handle).unwrap();
//                 let value = 1.0 - (visible_entity.order.0 - 10.0) / 7.0;
//                 material.albedo = Color::rgb(value, value, value);
//             }
//         }
//     }
// }
