use std::time::Instant;

use crystal_planes::map;
use crystal_planes::math::prelude::*;

fn bench(bm: &dyn map::Bitmap) {
    let t0 = Instant::now();
    let start = Point3i::new(40, 180, 40);
    let mut count = 0;
    let mut count_all = 0;
    for ((x, y, z), v) in bm.cell_iter() {
        count_all += 1;
        if *v {
            continue;
        }
        if bm.occluded(
            start,
            Point3i::new(x as i32, y as i32, z as i32),
            None,
            None,
        ) {
            count += 1;
        }
    }
    println!("count: {} {} {:?}", count, count_all, t0.elapsed());
}

fn main() {
    let bm = map::read_map("assets/maps/hidden_ramp.txt").expect("could not read file");
    bench(&*bm);
    // let bm = Box::new(crystal::accel::OctreeBitmap::wrap(bm));
    //bench(&*bm);
}
