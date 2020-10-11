use bevy::{
    prelude::*,
    render::{
        camera::{Camera, VisibleEntities},
        mesh::shape,
    },
};
fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_system(setup.system())
        .add_plugin(bevy_fly_camera::FlyCameraPlugin)
        // .add_system(camera_order_color_system.system())
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let cube_handle = meshes.add(Mesh::from(shape::Cube { size: 1.0 }));
    let cube_handle2 = meshes.add(Mesh::from(shape::Cube { size: 0.5 }));
    let cube_handle3 = meshes.add(Mesh::from(shape::Cube { size: 0.25 }));

    let cube_material_handle = materials.add(StandardMaterial {
        albedo: Color::rgba(0.5, 0.4, 0.3, 0.1),
        ..Default::default()
    });

    commands
        // parent cube
        .spawn(PbrComponents {
            mesh: cube_handle,
            material: cube_material_handle,
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
            draw: Draw {
                is_transparent: true,
                ..Default::default()
            },
            ..Default::default()
        })
        .spawn(PbrComponents {
            mesh: cube_handle2,
            material: cube_material_handle,
            transform: Transform::from_translation(Vec3::new(0.1, 0.1, 0.1)),
            draw: Draw {
                is_transparent: true,
                ..Default::default()
            },
            ..Default::default()
        })
        .spawn(PbrComponents {
            mesh: cube_handle3,
            material: cube_material_handle,
            transform: Transform::from_translation(Vec3::new(0.11, 0.11, 0.11)),
            draw: Draw {
                is_transparent: true,
                ..Default::default()
            },
            ..Default::default()
        })
        // light
        // .spawn(LightComponents {
        //     transform: Transform::from_translation(Vec3::new(4.0, 5.0, -4.0)),
        //     ..Default::default()
        // })
        // camera
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
            sensitivity: 8.0,
            ..Default::default()
        });
}

fn camera_order_color_system(
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut camera_query: Query<(&Camera, &VisibleEntities)>,
    material_query: Query<&Handle<StandardMaterial>>,
) {
    for (_camera, visible_entities) in &mut camera_query.iter() {
        for visible_entity in visible_entities.iter() {
            if let Ok(material_handle) =
                material_query.get::<Handle<StandardMaterial>>(visible_entity.entity)
            {
                let material = materials.get_mut(&material_handle).unwrap();
                let value = 1.0 - (visible_entity.order.0 - 10.0) / 7.0;
                material.albedo = Color::rgb(value, value, value);
            }
        }
    }
}
