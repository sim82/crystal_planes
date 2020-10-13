fn main() {
    let points: Vec<(u32, u32)> = vec![(1, 1), (1, 2), (2, 2), (3, 2)];

    for (x, y) in points {
        println!("{} {} {:b}", x, y, zorder2(x, y));
    }
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

    // println!("zorder: {} {} {:b}", x, y, out);
    out
}

#[test]
fn zorder2_test() {
    assert_eq!(zorder2(0, 0), 0);
    assert_eq!(zorder2(3, 5), 0b100111);
    assert_eq!(zorder2(6, 2), 0b011100);
    assert_eq!(zorder2(7, 7), 0b111111);
}
