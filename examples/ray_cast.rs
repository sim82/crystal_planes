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

fn min2(x: f32, y: f32) -> f32 {
    if x < y {
        x
    } else {
        y
    }
}
fn min3(x: f32, y: f32, z: f32) -> f32 {
    min2(min2(x, y), z)
}
fn min4(x: f32, y: f32, z: f32, w: f32) -> f32 {
    min3(min2(x, y), z, w)
}

fn max2(x: f32, y: f32) -> f32 {
    if x > y {
        x
    } else {
        y
    }
}

fn max3(x: f32, y: f32, z: f32) -> f32 {
    max2(max2(x, y), z)
}

fn max4(x: f32, y: f32, z: f32, w: f32) -> f32 {
    max3(max2(x, y), z, w)
}

fn cast_ray(octants: &octree::Octants, root: octree::OctantId, p: Vec3, d: Vec3) {
    // let epsilon = (-s_max)

    let tx_coef = -1f32 / d.x().abs();
    let ty_coef = -1f32 / d.y().abs();
    let tz_coef = -1f32 / d.z().abs();

    let tx_bias = tx_coef * p.x();
    let ty_bias = ty_coef * p.y();
    let tz_bias = tz_coef * p.z();

    // TODO: octant mirroring stuff
    assert!(d.x() <= 0f32);
    assert!(d.y() <= 0f32);
    assert!(d.z() <= 0f32);

    let root_octant = octants.get(root);
    let (root_pos, root_size) = root_octant.get_geometry(root_octant.scale);

    println!("root_geometry: {:?} {:?}", root_pos, root_size);
    assert!(root_pos.x() == root_pos.y() && root_pos.x() == root_pos.z());
    assert!(root_size.x() == root_size.y() && root_size.x() == root_size.z());
    let bounds_min = root_pos.x() as f32;
    let bounds_max = (root_pos.x() + root_size.x()) as f32;
    // todo: check if min/max stuff works in 'non 1-2 world' as in paper (or normalize voxel position/size to 1-2 or 0-1 space for casting)
    let mut t_min = max4(
        bounds_max * tx_coef - tx_bias,
        bounds_max * ty_coef - ty_bias,
        bounds_max * tz_coef - tz_bias,
        0f32,
    );
    let t_max = min3(
        bounds_min * tx_coef - tx_bias,
        bounds_min * ty_coef - ty_bias,
        bounds_min * ty_coef - ty_bias,
    );
    let mut h = t_max;
    let mut t_max = min2(t_max, 1f32);

    let mut parent = root;
    let mut idx = 0;
    let mut pos = Vec3::zero();
    assert!(octants.get(root).scale >= 1); // impossible in a well formed tree
    let mut scale = octants.get(root).scale - 1;
    let mut scale_exp2 = bounds_max / 2.0; // 2**(scale-s_max) == 0.5, 0.25, 0.125, ... in original paper

    let s_max = scale + 1;

    let mut stack = vec![(octree::OctantId::default(), 0f32); (s_max + 1) as usize];

    if 0.5 * bounds_max * tx_coef - tx_bias > t_min {
        idx ^= 1;
        *pos.x_mut() = 0.5 * bounds_max;
    };
    if 0.5 * bounds_max * ty_coef - ty_bias > t_min {
        idx ^= 2;
        *pos.y_mut() = 0.5 * bounds_max;
    };
    if 0.5 * bounds_max * tz_coef - tz_bias > t_min {
        idx ^= 4;
        *pos.z_mut() = 0.5 * bounds_max;
    };

    // return;

    while scale < s_max {
        let octant = octants.get(parent);

        let tx_corner = pos.x() * tx_coef - tx_bias;
        let ty_corner = pos.y() * ty_coef - ty_bias;
        let tz_corner = pos.z() * tz_coef - tz_bias;
        let tc_max = min3(tx_corner, ty_corner, tz_corner);

        println!(
            "idx: {} pos: {:?} t_corner: {:?} scale: {} tc_max: {} t_min: {} t_max: {}",
            idx,
            pos,
            (tx_corner, ty_corner, tz_corner),
            scale,
            tc_max,
            t_min,
            t_max
        );

        if octant.children[idx] != octree::Voxel::Empty && t_min < t_max {
            let tv_max = min2(t_max, tc_max);
            let half = scale_exp2 * 0.5f32;
            // let half = bounds_max * 0.5;
            let tx_center = half * tx_coef + tx_corner;
            let ty_center = half * ty_coef + ty_corner;
            let tz_center = half * tz_coef + tz_corner;
            println!("t_min: {} t_max: {} tv_max: {}", t_min, t_max, tv_max);
            if t_min <= tv_max {
                if octant.children[idx] == octree::Voxel::Leaf {
                    println!("reached leaf");
                    break;
                } else if let octree::Voxel::Octant(child_id) = octant.children[idx] {
                    // PUSH
                    println!("push");

                    if tc_max < h {
                        stack[scale as usize] = (parent, t_max);
                    }
                    h = tc_max;

                    parent = child_id;
                    idx = 0;
                    scale -= 1;
                    scale_exp2 = half;
                    if tx_center > t_min {
                        idx ^= 1;
                        *pos.x_mut() += scale_exp2;
                    }
                    if ty_center > t_min {
                        idx ^= 2;
                        *pos.y_mut() += scale_exp2;
                    }
                    if tz_center > t_min {
                        idx ^= 4;
                        *pos.z_mut() += scale_exp2;
                    }

                    t_max = tv_max;
                    continue;
                } else {
                    panic!("unreachable");
                }
            }
        }
        // ADVANCE
        println!("advance");
        let mut step_mask = 0;
        if tx_corner <= tc_max {
            step_mask ^= 1;
            *pos.x_mut() -= scale_exp2;
        }
        if ty_corner <= tc_max {
            step_mask ^= 2;
            *pos.y_mut() -= scale_exp2;
        }
        if tz_corner <= tc_max {
            step_mask ^= 4;
            *pos.z_mut() -= scale_exp2;
        }
        t_min = tc_max;
        idx ^= step_mask;

        println!("idx: {} pos: {:?}", idx, pos);

        if idx & step_mask != 0 {
            // POP
            // Find the highest differing bit between the two positions.
            let mut differing_bits = 0;
            if (step_mask & 1) != 0 {
                println!(
                    "x differing bits: {:b} {:b}",
                    (pos.x() as i32),
                    (pos.x() + scale_exp2) as i32
                );

                differing_bits |= (pos.x() as i32) ^ ((pos.x() + scale_exp2) as i32)
            }
            if (step_mask & 2) != 0 {
                println!(
                    "y differing bits: {} {}",
                    (pos.y() as i32),
                    (pos.y() + scale_exp2) as i32
                );
                differing_bits |= (pos.y() as i32) ^ ((pos.y() + scale_exp2) as i32)
            }
            if (step_mask & 4) != 0 {
                println!(
                    "z differing bits: {} {}",
                    (pos.z() as i32),
                    (pos.z() + scale_exp2) as i32
                );
                differing_bits |= (pos.z() as i32) ^ ((pos.z() + scale_exp2) as i32)
            }
            println!(
                "differing bits: {:b} {}",
                differing_bits,
                differing_bits.leading_zeros()
            );
            scale = 31 - differing_bits.leading_zeros();
            parent = stack[scale as usize].0;
            t_max = stack[scale as usize].1;
            println!(
                "scale: {} parent: {:?} t_max: {}",
                scale,
                octants.get(parent),
                t_max
            );

            // Round cube position and extract child slot index.
            let shx = (pos.x() as i32) >> scale;
            let shy = (pos.y() as i32) >> scale;
            let shz = (pos.z() as i32) >> scale;
            *pos.x_mut() = (shx << scale) as f32;
            *pos.y_mut() = (shy << scale) as f32;
            *pos.z_mut() = (shz << scale) as f32;
            idx = ((shx & 1) | ((shy & 1) << 1) | ((shz & 1) << 2)) as usize;
            // Prevent same parent from being stored again and invalidate cached child descriptor.
            h = 0f32;

            panic!("pop");
        }
    }
}

#[test]
fn test_cast1() {
    let mut octants = octree::Octants::default();

    // let mut points = Vec::new();
    // for z in 0..16 {
    //     for y in 0..16 {
    //         for x in 0..16 {
    //             points.push(Point3i::new(x, y, z));
    //         }
    //     }
    // }

    let points = vec![
        Point3i::new(0, 0, 0),
        Point3i::new(0, 11, 11),
        Point3i::new(15, 15, 15),
    ];

    // let points: Vec<Vec3i> = (0..16)
    //     .zip((0..16).zip(0..16))
    //     .map(|(x, (y, z))| Point3i::new(x, y, z))
    //     .collect();
    // let points = [
    //     Point3i::new(0, 0, 0),
    //     Point3i::new(15, 8, 8),
    //     Point3i::new(15, 15, 15),
    // ];
    let root =
        octree::create_octants_bottom_up(&mut octants, &points).expect("failed to create octree");

    println!("root: {:?}", root);
    for o in octants.octants.iter() {
        println!("{:?}", o);
    }

    cast_ray(
        &octants,
        root,
        Vec3::new(20f32, 12f32, 12f32),
        Vec3::new(-20f32, -0.001f32, -0.001f32),
    );

    // cast_ray(
    //     &octants,
    //     root,
    //     Vec3::new(20f32, 4f32, 4f32),
    //     Vec3::new(-20f32, -0.1f32, -0.1f32),
    // );

    // cast_ray(
    //     &octants,
    //     root,
    //     Vec3::new(20f32, 2f32, 2f32),
    //     Vec3::new(-8f32, -0.1f32, -0.1f32),
    // );

    // cast_ray(
    //     &octants,
    //     root,
    //     Vec3::new(12f32, 2f32, 2f32),
    //     Vec3::new(-8f32, -0.1f32, -0.1f32),
    // );
}

#[test]
fn test_raycast_math() {
    test(
        Vec3::new(20f32, 4f32, 4f32),
        Vec3::new(-8f32, -0.1f32, -0.1f32),
    );
    test(
        Vec3::new(20f32, 12f32, 4f32),
        Vec3::new(-8f32, -0.1f32, -0.1f32),
    );
    test(
        Vec3::new(20f32, 4f32, 12f32),
        Vec3::new(-8f32, -0.1f32, -0.1f32),
    );
    test(
        Vec3::new(20f32, 12f32, 12f32),
        Vec3::new(-8f32, -0.1f32, -0.1f32),
    );
    test(
        Vec3::new(4f32, 20f32, 4f32),
        Vec3::new(-0.1f32, -8f32, -0.1f32),
    );
    fn test(p: Vec3, d: Vec3) {
        let tx_coef = -1f32 / d.x().abs();
        let ty_coef = -1f32 / d.y().abs();
        let tz_coef = -1f32 / d.z().abs();

        let tx_bias = tx_coef * p.x();
        let ty_bias = ty_coef * p.y();
        let tz_bias = tz_coef * p.z();

        // let t_coef = Vec3::new(1f32 / d.x().abs(), 1f32 / d.y().abs(), 1f32 / d.z().abs());
        // let t_bias = t_coef * p;

        println!(
            "coef: {:?} bias: {:?}",
            (tx_coef, ty_coef, tz_coef),
            (tx_bias, ty_bias, tz_bias)
        );

        // TODO: octant mirroring stuff
        assert!(d.x() <= 0f32);
        assert!(d.y() <= 0f32);
        assert!(d.z() <= 0f32);

        let bounds_min = 0f32;
        let bounds_max = 16f32;
        // todo: check if min/max stuff works in 'non 1-2 world' as in paper (or normalize voxel position/size to 1-2 or 0-1 space for casting)
        let t_min = max4(
            bounds_max * tx_coef - tx_bias,
            bounds_max * ty_coef - ty_bias,
            bounds_max * tz_coef - tz_bias,
            0f32,
        );
        let t_max = min3(
            bounds_min * tx_coef - tx_bias,
            bounds_min * ty_coef - ty_bias,
            bounds_min * ty_coef - ty_bias,
        );
        let mut h = t_max;
        let mut t_max = min2(t_max, 1f32);

        println!("t_min: {} t_max: {}", t_min, t_max);
        let mut idx = 0;
        let mut pos = Vec3::zero();
        if 0.5 * bounds_max * tx_coef - tx_bias > t_min {
            idx ^= 1;
            *pos.x_mut() = 0.5 * bounds_max;
        };
        if 0.5 * bounds_max * ty_coef - ty_bias > t_min {
            idx ^= 2;
            *pos.y_mut() = 0.5 * bounds_max;
        };
        if 0.5 * bounds_max * tz_coef - tz_bias > t_min {
            idx ^= 4;
            *pos.z_mut() = 0.5 * bounds_max;
        };

        println!("idx: {} pos: {:?}", idx, pos);
    }
}
