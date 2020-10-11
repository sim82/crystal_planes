use crystal_planes::crystal;
use crystal_planes::crystal::math::*;

fn main() {
    let bm = crystal::read_map("assets/maps/hidden_ramp.txt").expect("could not read file");

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
    let mut zordered: Vec<_> = points.iter().map(|p| zorder(p)).collect();
    zordered.sort();
    for i in zordered {
        println!("{:24b}", i);
    }
    // for p in points.iter() {
    //     println!("{:b}", zorder(p));
    // }
}

fn zorder(p: &Point3i) -> usize {
    assert!(p.x() >= 0 && p.y() >= 0 && p.z() >= 0);
    let mut x = p.x() as u32;
    let mut y = p.y() as u32;
    let mut z = p.z() as u32;

    let mut rout: usize = 0;
    let mut n = 0;
    while x > 0 || y > 0 || z > 0 {
        rout <<= 1;
        rout |= (z & 0b1) as usize;
        rout <<= 1;
        rout |= (y & 0b1) as usize;
        rout <<= 1;
        rout |= (x & 0b1) as usize;
        x >>= 1;
        y >>= 1;
        z >>= 1;
        n += 1;
    }
    let mut out = 0;
    for _ in 0..n {
        out <<= 3;
        out |= rout & 0b111;
        rout >>= 3;
    }
    // println!("zorder: {:?} {:9b}", p, out);
    out
}

#[test]
fn zorder_test() {
    assert_eq!(zorder(&Point3i::new(0, 0, 0)), 0);
    assert_eq!(zorder(&Point3i::new(1, 0, 0)), 0b1);
    assert_eq!(zorder(&Point3i::new(0, 1, 0)), 0b10);
    assert_eq!(zorder(&Point3i::new(0, 0, 1)), 0b100);
    assert_eq!(zorder(&Point3i::new(7, 0, 0)), 0b1001001);
    assert_eq!(zorder(&Point3i::new(0, 7, 0)), 0b10010010);
    assert_eq!(zorder(&Point3i::new(0, 0, 7)), 0b100100100);
    assert_eq!(zorder(&Point3i::new(7, 0, 0)), 0b1001001);
    assert_eq!(zorder(&Point3i::new(3, 5, 0)), 0b10001011);
    assert_eq!(zorder(&Point3i::new(3, 5, 7)), 0b110101111);
    assert_eq!(zorder(&Point3i::new(1, 2, 3)), 0b110101);
    assert_eq!(zorder(&Point3i::new(7, 7, 7)), 0b111111111);
}

fn zorder2(mut x: u32, mut y: u32) -> usize {
    let mut rout: usize = 0;

    let mut n = 0;
    while x > 0 || y > 0 {
        rout <<= 1;
        rout |= (y & 0b1) as usize;
        rout <<= 1;
        rout |= (x & 0b1) as usize;
        x >>= 1;
        y >>= 1;
        n += 1;
    }
    let mut out = 0;
    for _ in 0..n {
        out <<= 2;
        out |= (rout & 0b11) as usize;
        rout >>= 2;
    }

    println!("zorder: {} {} {:b}", x, y, out);
    out
}

#[test]
fn zorder2_test() {
    assert_eq!(zorder2(0, 0), 0);
    assert_eq!(zorder2(3, 5), 0b100111);
    assert_eq!(zorder2(6, 2), 0b011100);
    assert_eq!(zorder2(7, 7), 0b111111);
}
