use std::time::Instant;

use crystal_planes::map;
use crystal_planes::math::prelude::*;
use crystal_planes::octree;

fn main() {
    let bm = map::read_map("assets/maps/hidden_ramp.txt").expect("could not read file");

    let points: Vec<Point3i> = bm
        .cell_iter()
        .filter_map(|((x, y, z), v)| {
            if *v {
                Some(Point3i::new(x as i32, y as i32, z as i32))
            } else {
                None
            }
        })
        .collect();
    // let points = vec![
    //     Vec3i::new(0, 0, 0),
    //     Vec3i::new(1, 1, 1),
    //     Vec3i::new(2, 0, 0),
    //     Vec3i::new(3, 1, 1),
    //     Vec3i::new(0, 2, 0),
    //     Vec3i::new(1, 3, 1),
    //     Vec3i::new(2, 2, 2),
    //     Vec3i::new(0, 0, 2),
    //     Vec3i::new(3, 3, 3),
    //     Vec3i::new(10, 10, 10),
    //     Vec3i::new(11, 11, 11),
    // ];

    // let mut zordered: Vec<_> = points.iter().map(|p| octree::zorder(p)).collect();
    // zordered.sort();
    // // for i in zordered {
    //     println!("{:24b}", i);
    // }
    // for p in points.iter() {
    //     println!("{:b}", zorder(p));
    // }

    // for p in points.iter() {
    //     // println!("{:?}\t{:24b}", p, zorder(p));
    //     println!("{:?}\t{:8o}", p, octree::zorder(p));
    // }
    let start = Instant::now();
    let mut octants = octree::Octants::default();
    let _root = octree::create_octants_bottom_up(&mut octants, &points);
    println!("time: {:?}", start.elapsed());
    // for octant in octants.octants {
    //     println!("{:?}", octant);
    // }
}
