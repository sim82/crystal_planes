use crate::crystal::map;
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

use super::Bitmap;
pub struct OctreeBitmap {
    bitmap: Box<dyn Bitmap + Sync + Send>,
    octree: OctreeDBVT<i32>,
}

#[derive(Clone)]
struct Voxel(bool);
impl IsEmpty for Voxel {
    fn is_empty(&self) -> bool {
        !self.0
    }
}

impl OctreeBitmap {
    pub fn wrap(bitmap: Box<dyn Bitmap + Sync + Send>) -> Self {
        let points: Vec<Point3i> = bitmap
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

        OctreeBitmap {
            bitmap,
            octree: bvt,
        }
    }
}

impl Bitmap for OctreeBitmap {
    fn set(&mut self, p: super::math::Point3i, v: bool) {
        self.bitmap.set(p, v)
    }

    fn get(&self, p: super::math::Point3i) -> bool {
        self.bitmap.get(p)
    }

    fn add(&mut self, slice: &map::MapSlice) {
        self.bitmap.add(slice)
    }

    fn print(&self) {
        self.bitmap.print();
    }

    fn step(&self, p: super::math::Point3i, dir: &super::Dir) -> Option<super::math::Point3i> {
        self.bitmap.step(p, dir)
    }

    fn cell_iter(&self) -> ndarray::iter::IndexedIter<'_, bool, ndarray::Ix3> {
        self.bitmap.cell_iter()
    }

    fn occluded(
        &self,
        p0: super::math::Vec3i,
        p1: super::math::Vec3i,
        n0: Option<super::math::Vec3i>,
        n1: Option<super::math::Vec3i>,
    ) -> bool {
        let p0 = match n0 {
            Some(n0) => p0.into_vec3() + n0.into_vec3(),
            _ => p0.into_vec3(),
        };
        let p1 = match n1 {
            Some(n1) => p1.into_vec3() + n1.into_vec3(),
            _ => p1.into_vec3(),
        };
        let start = na::Point3::new(p0.x() + 0.5, p0.y() + 0.5, p0.z() + 0.5);
        let end = na::Point3::new(p1.x() + 0.5, p1.y() + 0.5, p1.z() + 0.5);

        let ray = Ray::new(start, end - start);
        let impact = voxel_ray_cast(&self.octree, ray, std::f32::MAX, |_| true);
        impact.is_some()
    }
}
