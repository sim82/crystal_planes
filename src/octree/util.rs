use crate::map;
use crate::math::prelude::*;

use crate::octree;

pub trait OctreeLoad {
    fn load_map(&mut self, filename: &str) -> Option<octree::OctantId>;
}

impl OctreeLoad for crate::octree::Octants {
    fn load_map(&mut self, filename: &str) -> Option<octree::OctantId> {
        let bm = map::read_map(filename).ok()?;

        let points: Vec<Point3i> = bm
            .cell_iter()
            .filter_map(|((x, y, z), v)| {
                if v {
                    Some(Point3i::new(x as i32, y as i32, z as i32))
                } else {
                    None
                }
            })
            .collect();
        octree::create_octants_bottom_up(self, &points)
    }
}
