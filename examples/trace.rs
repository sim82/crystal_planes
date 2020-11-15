use std::time::Instant;

use crystal_planes::map;
use crystal_planes::map::DenseBlockmap;
use crystal_planes::math::prelude::*;
fn bench(bm: &dyn map::Bitmap, cells: &[Point3i]) {
    let t0 = Instant::now();
    let start = Point3i::new(40, 5, 40);
    let mut count = 0;
    let mut count_all = 0;
    for _ in 0..100 {
        for p in cells.iter() {
            count_all += 1;
            if bm.occluded(start, Point3i::new(p.x(), p.y(), p.z()), None, None, true) {
                count += 1;
            }
        }
    }
    println!("count: {} {} {:?}", count, count_all, t0.elapsed());
}

fn main() {
    let bm = map::read_map("assets/maps/hidden_ramp.txt").expect("could not read file");
    let dbm = DenseBlockmap::from_bitmap(&*bm);
    let cells: Vec<_> = bm
        .cell_iter()
        .map(|((x, y, z), v)| Point3i::new(x as i32, y as i32, z as i32))
        .collect();

    // bench(&*bm, &cells[..]);
    bench(&dbm, &cells[..]);
    // let bm = Box::new(crystal::accel::OctreeBitmap::wrap(bm));
    //bench(&*bm);
}
