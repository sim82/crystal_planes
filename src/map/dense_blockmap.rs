use crate::map;
use crate::map::Bitmap;
use crate::math::prelude::*;
use crate::util;

struct DenseBlockmap {
    x: usize,
    y: usize,
    z: usize,
    xi: usize,
    yi: usize,
    zi: usize,

    blocks: Vec<u64>,
}

impl DenseBlockmap {
    pub fn new(x: usize, y: usize, z: usize) -> Self {
        fn num_blocks(s: usize) -> usize {
            s / 4 + if s % 4 != 0 { 1 } else { 0 }
        }
        let xi = num_blocks(x);
        let yi = num_blocks(y);
        let zi = num_blocks(z);
        DenseBlockmap {
            x,
            y,
            z,
            xi,
            yi,
            zi,
            blocks: vec![0; xi * yi * zi],
        }
    }
    pub fn from_bitmap(bm: &dyn Bitmap) -> Self {
        // FIXME: this is crap... just add a method to retrieve size...
        let x: Vec<_> = bm
            .cell_iter()
            .filter_map(|((x, y, z), v)| if *v { Some((x, y, z)) } else { None })
            .collect();

        let mut xm = 0;
        let mut ym = 0;
        let mut zm = 0;

        for (x, y, z) in x.iter() {
            xm = xm.max(*x);
            ym = ym.max(*y);
            zm = zm.max(*z);
        }
        let mut out = DenseBlockmap::new(xm + 1, ym + 1, zm + 1);
        for (x, y, z) in x {
            out.set(x, y, z, true);
        }
        out
    }

    fn block_address(&self, x: usize, y: usize, z: usize) -> (usize, usize) {
        let xm = x % 4;
        let ym = y % 4;
        let zm = z % 4;

        let x = x / 4;
        let y = y / 4;
        let z = z / 4;

        (
            x + y * self.xi + z * self.xi * self.yi,
            xm + ym * 4 + zm * 4 * 4,
        )
    }
    pub fn get(&self, x: usize, y: usize, z: usize) -> bool {
        let (block, bit) = self.block_address(x, y, z);
        self.blocks[block] & (0b1 << bit) != 0
    }
    pub fn set(&mut self, x: usize, y: usize, z: usize, v: bool) {
        let (block, bit) = self.block_address(x, y, z);
        let mask = 0b1 << bit;
        if v {
            self.blocks[block] |= mask
        } else {
            self.blocks[block] &= !mask
        }
    }
}

impl Bitmap for DenseBlockmap {
    fn get(&self, p: Point3i) -> bool {
        //self.get(p.x(), p.y().p.z())
        let x = p.x();
        let y = p.y();
        let z = p.z();

        if x < 0
            || y < 0
            || z < 0
            || x as usize >= self.x
            || y as usize >= self.y
            || z as usize >= self.z
        {
            return false;
        }
        DenseBlockmap::get(self, x as usize, y as usize, z as usize)
    }

    fn print(&self) {
        todo!()
    }

    fn step(&self, p: Point3i, dir: &super::Dir) -> Option<Point3i> {
        let pnew = p + dir.get_normal_i();
        if pnew.x() < 0
            || pnew.y() < 0
            || pnew.z() < 0
            || pnew.x() >= self.x as i32
            || pnew.y() >= self.y as i32
            || pnew.z() >= self.z as i32
        {
            None
        } else {
            Some(pnew)
        }
    }

    fn cell_iter(&self) -> Box<dyn Iterator<Item = ((usize, usize, usize), &bool)> + '_> {
        todo!()
    }

    fn occluded(
        &self,
        p0: Vec3i,
        p1: Vec3i,
        n0: Option<Vec3i>,
        n1: Option<Vec3i>,
        from_inside: bool,
    ) -> bool {
        if from_inside {
            let min = Vec3i::zero();
            let max = Vec3i::new(self.x as i32, self.y as i32, self.z as i32);
            match (n0, n1) {
                (Some(n0), Some(n1)) => {
                    util::occluded_from_inside(p0 + n0, p1 + n1, self, min, max)
                }
                _ => util::occluded_from_inside(p0, p1, self, min, max),
            }
        } else {
            match (n0, n1) {
                (Some(n0), Some(n1)) => util::occluded(p0 + n0, p1 + n1, self),
                _ => util::occluded(p0, p1, self),
            }
        }
    }
}

#[test]
fn test_basic() {
    let mut bm = DenseBlockmap::new(1024, 1024, 1024);

    bm.set(666, 123, 731, true);
    assert!(bm.get(666, 123, 731));
    assert!(!bm.get(666, 123, 732));
    assert!(!bm.get(667, 123, 732));
    assert!(!bm.get(666, 124, 732));
}

#[test]
fn trace() {
    let bm = map::read_map("assets/maps/hidden_ramp.txt").expect("could not read file");
    let dbm = DenseBlockmap::from_bitmap(&*bm);

    for ((x, y, z), v) in bm.cell_iter() {
        assert_eq!(dbm.get(x, y, z), *v);
    }
}
