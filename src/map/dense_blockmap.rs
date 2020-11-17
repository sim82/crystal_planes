use std::time::Instant;

use crate::map;
use crate::map::Bitmap;
use crate::math::prelude::*;
use crate::util;

pub struct DenseBlockmap {
    x: usize,
    y: usize,
    z: usize,
    xi: usize,
    xyi: usize,

    blocks: Vec<u64>,
    tmp: bool,
}

struct Iter<'a> {
    bm: &'a DenseBlockmap,
    ix: usize,
    iy: usize,
    iz: usize,
}

impl<'a> Iter<'a> {
    pub fn new(bm: &'a DenseBlockmap) -> Self {
        Self {
            bm: bm,
            ix: 0,
            iy: 0,
            iz: 0,
        }
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = ((usize, usize, usize), bool);

    fn next(&mut self) -> Option<Self::Item> {
        if self.iz >= self.bm.z {
            return None;
        }

        let ret = Some((
            (self.ix, self.iy, self.iz),
            self.bm.get(self.ix, self.iy, self.iz)?,
        ));

        self.ix += 1;
        if self.ix >= self.bm.x {
            self.iy += 1;
            self.ix = 0;
        }
        if self.iy >= self.bm.y {
            self.iz += 1;
            self.iy = 0;
        }
        ret
    }
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
            xyi: xi * yi,
            blocks: vec![0; xi * yi * zi],
            tmp: false,
        }
    }
    pub fn from_bitmap(bm: &dyn Bitmap) -> Self {
        // FIXME: this is crap... just add a method to retrieve size...
        let x: Vec<_> = bm
            .cell_iter()
            .filter_map(|((x, y, z), v)| if v { Some((x, y, z)) } else { None })
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
        for (x, y, z) in x.iter() {
            out.set(*x, *y, *z, true);
        }
        out
    }

    fn block_address(&self, x: usize, y: usize, z: usize) -> Option<usize> {
        let x = x / 4;
        let y = y / 4;
        let z = z / 4;
        let block = x + y * self.xi + z * self.xyi;
        if block >= self.blocks.len() {
            return None;
        }
        Some(block)
    }
    fn block_address_unchecked(&self, x: usize, y: usize, z: usize) -> usize {
        let x = x / 4;
        let y = y / 4;
        let z = z / 4;
        x + y * self.xi + z * self.xyi
    }
    fn block_mask(&self, x: usize, y: usize, z: usize) -> u64 {
        let xm = x % 4;
        let ym = y % 4;
        let zm = z % 4;

        // const ZI: [u64; 4] = [0x1, 0x00001, 0x000000001, 0x0000000000001]; // might help on cpus with slow shift (if such a thing still exists...)
        0b1 << (xm + ym * 4 + zm * 4 * 4)
        // ZI[zm] << (xm + ym * 4),
    }
    pub fn get(&self, x: usize, y: usize, z: usize) -> Option<bool> {
        match self.block_address(x, y, z) {
            Some(block) => Some({
                // SAFE because bounds check is done in block_address
                let bits = unsafe { *self.blocks.get_unchecked(block) };
                (bits != 0) && (bits & self.block_mask(x, y, z)) != 0
            }),
            None => None,
        }
    }
    unsafe fn get_unchecked(&self, x: usize, y: usize, z: usize) -> bool {
        let block = self.block_address_unchecked(x, y, z);
        let bits = self.blocks.get_unchecked(block);
        (*bits != 0) && (bits & self.block_mask(x, y, z)) != 0
    }

    pub fn set(&mut self, x: usize, y: usize, z: usize, v: bool) {
        match self.block_address(x, y, z) {
            None => (),
            Some(block) => {
                if v {
                    self.blocks[block] |= self.block_mask(x, y, z)
                } else {
                    self.blocks[block] &= !self.block_mask(x, y, z)
                }
            }
        }
        // let (block, mask) = self.block_address(x, y, z);
        // if v {
        //     self.blocks[block] |= mask
        // } else {
        //     self.blocks[block] &= !mask
        // }
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
        // SAFE due to bounds check above
        unsafe { DenseBlockmap::get_unchecked(self, x as usize, y as usize, z as usize) }
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

    fn cell_iter(&self) -> Box<dyn Iterator<Item = ((usize, usize, usize), bool)> + '_> {
        Box::new(Iter::new(self))
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
    assert!(bm.get(666, 123, 731).unwrap());
    assert!(!bm.get(666, 123, 732).unwrap());
    assert!(!bm.get(667, 123, 732).unwrap());
    assert!(!bm.get(666, 124, 732).unwrap());
}

fn bench(bm: &dyn map::Bitmap, cells: &[Point3i]) {
    let t0 = Instant::now();
    let start = Point3i::new(40, 180, 40);
    let mut count = 0;
    let mut count_all = 0;
    for p in cells.iter() {
        count_all += 1;
        if bm.occluded(Point3i::new(p.x(), p.y(), p.z()), start, None, None, true) {
            count += 1;
        }
    }
    println!("count: {} {} {:?}", count, count_all, t0.elapsed());
}
#[test]
fn trace() {
    let bm = map::read_map("assets/maps/hidden_ramp.txt").expect("could not read file");
    let dbm = DenseBlockmap::from_bitmap(&*bm);

    let mut cells = Vec::new();
    for ((x, y, z), v) in bm.cell_iter() {
        assert_eq!(dbm.get(x, y, z).unwrap(), v);
        cells.push(Point3i::new(x as i32, y as i32, z as i32));
    }

    bench(&*bm, &cells[..]);
    bench(&dbm, &cells[..]);
}
