use bevy::{
    prelude::*,
    render::{
        camera::{Camera, VisibleEntities},
        mesh::shape,
    },
};
use building_blocks::{
    core::prelude::*,
    partition::collision::voxel_ray_cast,
    partition::ncollide3d::{
        na,
        query::{Ray, RayCast},
    },
    partition::{Octree, OctreeDBVT, OctreeDBVTVisitor, OctreeVisitor},
    prelude::*,
    storage::prelude::*,
};
use crystal_planes::crystal;

use image::{png::PngEncoder, ColorType};
use rand::{thread_rng, Rng};
use std::fs::File;

#[derive(Clone)]
struct Voxel(bool);

impl IsEmpty for Voxel {
    fn is_empty(&self) -> bool {
        !self.0
    }
}

struct DebugVisitor;
impl OctreeVisitor for DebugVisitor {
    fn visit_octant(
        &mut self,
        octant: building_blocks::partition::Octant,
        is_leaf: bool,
    ) -> building_blocks::partition::octree::VisitStatus {
        println!(
            "visit: {:?} {:?} {}",
            is_leaf, octant.minimum, octant.edge_length
        );
        building_blocks::partition::octree::VisitStatus::Continue
    }
}

struct DebugVisitorDbvt;
impl OctreeDBVTVisitor for DebugVisitorDbvt {
    fn visit(
        &mut self,
        aabb: &building_blocks::partition::ncollide3d::bounding_volume::AABB<f32>,
        octant: Option<&building_blocks::partition::Octant>,
        is_leaf: bool,
    ) -> building_blocks::partition::octree::VisitStatus {
        match octant {
            Some(octant) => println!(
                "visit octant: {:?} {:?} {}",
                is_leaf, octant.minimum, octant.edge_length
            ),
            None => println!("visit None: {:?}", is_leaf),
        };
        building_blocks::partition::octree::VisitStatus::Continue
    }
}

fn main() {
    let bm = crystal::read_map("assets/maps/hidden_ramp.txt").expect("could not read file");
    let points: Vec<Point3i> = bm
        .cell_iter()
        .filter_map(|((x, y, z), v)| {
            if *v {
                Some(PointN([x as i32, y as i32, z as i32]))
            } else {
                None
            }
        })
        .collect();

    let extents = [
        Extent3i::from_min_and_shape(PointN([0; 3]), PointN([64; 3])),
        Extent3i::from_min_and_shape(PointN([64, 0, 0]), PointN([64; 3])),
        Extent3i::from_min_and_shape(PointN([0, 64, 0]), PointN([64; 3])),
        Extent3i::from_min_and_shape(PointN([0, 0, 64]), PointN([64; 3])),
        Extent3i::from_min_and_shape(PointN([64, 64, 0]), PointN([64; 3])),
        Extent3i::from_min_and_shape(PointN([0, 64, 64]), PointN([64; 3])),
        Extent3i::from_min_and_shape(PointN([64, 0, 64]), PointN([64; 3])),
        Extent3i::from_min_and_shape(PointN([64, 64, 64]), PointN([64; 3])),
    ];

    let mut arrays: Vec<_> = extents
        .iter()
        .map(|e| (e.clone(), Array3::fill(e.clone(), Voxel(false))))
        .collect();
    // let mut voxels = Array3::fill(extent, Voxel(false));
    for p in points.iter() {
        for (ext, arr) in arrays.iter_mut() {
            if ext.contains(p) {
                // println!("{:?}", p);
                *arr.get_mut(p) = Voxel(true);
                // break;
            }
        }
        // *voxels.get_mut(p) = Voxel(true);
    }

    // let octree = Octree::from_array3(&voxels, *voxels.extent());
    let mut bvt = OctreeDBVT::default();
    let mut key = 0; // unimportant
    for (_, arr) in arrays {
        let octree = Octree::from_array3(&arr, *arr.extent());
        // println!("insert: {:?} {:?}", arr.extent(), octree.);

        // println!("new octree {:?}", arr.extent());
        // octree.visit(&mut DebugVisitor);
        bvt.insert(key, octree);
        key += 1;
    }
    bvt.visit(&mut DebugVisitorDbvt);

    {
        let start = na::Point3::new(40.0, 40.0, 40.0);
        let ray = Ray::new(start, na::Point3::new(40.0, 0.0, 40.0) - start);
        let impact = voxel_ray_cast(&bvt, ray, std::f32::MAX, |_| true);
        match impact {
            Some(impact) => println!("impact: {:?}", impact),
            None => println!("miss"),
        };
    }
    let mut pixels = [0u8; 128 * 128];
    for y in 0..128 {
        for x in 0..128 {
            // let start = na::Point3::new(x as f32, 40.0, y as f32);
            // let ray = Ray::new(start, na::Point3::new(x as f32, 0.0, y as f32) - start);
            let start = na::Point3::new(x as f32, y as f32, 128.0);
            let ray = Ray::new(start, na::Vector3::new(0.0, 0.0, -128.0));
            // let start = na::Point3::new(x as f32, y as f32, 128.0);
            // let ray = Ray::new(start, na::Vector3::new(0.0, -128.0, -128.0));
            let impact = voxel_ray_cast(&bvt, ray, std::f32::MAX, |_| true);
            match impact {
                Some(impact) => pixels[x + y * 128] = ((1.0 - impact.impact.toi) * 255.0) as u8,
                None => (),
            };
        }
    }

    let output = File::create("hit.png").unwrap();
    PngEncoder::new(output)
        .encode(&pixels, 128, 128, ColorType::L8)
        .unwrap();
    // bvt.insert(key, octree);

    // partition::Octree::from_array3(array, extent)
    // App::build()
    //     .add_plugins(DefaultPlugins)
    //     .add_startup_system(setup.system())
    //     .add_plugin(bevy_fly_camera::FlyCameraPlugin)
    //     .init_resource::<octree::Octants>()
    //     // .add_system(camera_order_color_system.system())
    //     .run();
}
