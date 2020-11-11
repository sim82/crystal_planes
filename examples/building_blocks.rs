use bevy::{
    prelude::*,
    render::{
        camera::{Camera, VisibleEntities},
        mesh::shape,
    },
};
use building_blocks::{
    core::prelude::*,
    partition::{Octree, OctreeDBVT},
    prelude::*,
    storage::prelude::*,
};
use crystal_planes::crystal;

use rand::{thread_rng, Rng};

#[derive(Clone)]
struct Voxel(bool);

impl IsEmpty for Voxel {
    fn is_empty(&self) -> bool {
        !self.0
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
                *arr.get_mut(p) = Voxel(true);
            }
        }
        // *voxels.get_mut(p) = Voxel(true);
    }

    // let octree = Octree::from_array3(&voxels, *voxels.extent());
    let mut bvt = OctreeDBVT::default();
    let key = 0; // unimportant
    for (_, arr) in arrays {
        bvt.insert(key, Octree::from_array3(&arr, *arr.extent()));
    }

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
