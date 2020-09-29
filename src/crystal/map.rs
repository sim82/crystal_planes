use crate::crystal::math::prelude::*;
use bevy::prelude::*;
use core::{fmt, ops::*};
use std::{
    fs::File,
    io::{BufRead, BufReader},
    iter::Iterator,
    path::Path,
    sync::Arc,
};

use crate::crystal::math::prelude::*;

pub type BlockMap = ndarray::Array3<bool>;
const NUM_PLANE_CORNERS: usize = 4;

pub trait Bitmap {
    fn set(&mut self, p: Point3i, v: bool);

    fn get(&self, p: Point3i) -> bool;

    fn add(&mut self, slice: &MapSlice);

    fn print(&self);
    fn step(&self, p: Point3i, dir: &Dir) -> Option<Point3i>;

    fn cell_iter(&self) -> ndarray::iter::IndexedIter<'_, bool, ndarray::Ix3>; // FIXME: hide this
}

impl Bitmap for BlockMap {
    fn set(&mut self, p: Point3i, v: bool) {
        // let c = self.coord(&p);
        self[[p.0 as usize, p.1 as usize, p.2 as usize]] = v;
    }

    fn get(&self, p: Point3i) -> bool {
        let (x, y, z) = self.dim();
        if p.x() < 0
            || p.y() < 0
            || p.z() < 0
            || p.x() as usize >= x
            || p.y() as usize >= y
            || p.z() as usize >= z
        {
            return false;
        }
        // self.bitmap[self.coord(&p)]
        self[[p.x() as usize, p.y() as usize, p.z() as usize]]
    }

    fn add(&mut self, slice: &MapSlice) {
        let (sx, _, sz) = self.dim();

        //let Vec2i { x: w, y: h } = slice.size();
        let size = slice.size();
        let w = size.x();
        let h = size.y();
        assert!(w >= sx as i32);
        assert!(h >= sz as i32);

        for ((x, y, z), v) in self.indexed_iter_mut() {
            *v = slice.get(Point2i::new(x as i32, z as i32)) >= y as i32;
        }
    }

    fn print(&self) {
        for xz_slice in self.axis_iter(ndarray::Axis(1)) {
            for x_slice in xz_slice.axis_iter(ndarray::Axis(0)) {
                for x in x_slice.iter() {
                    print!("{}", if *x { 1 } else { 0 });
                }
                println!();
            }
            println!("===================================");
        }
    }

    fn step(&self, p: Point3i, dir: &Dir) -> Option<Point3i> {
        let (x, y, z) = self.dim();
        let pnew = p + dir.get_normal_i();
        if pnew.x() < 0
            || pnew.y() < 0
            || pnew.z() < 0
            || pnew.x() >= x as i32
            || pnew.y() >= y as i32
            || pnew.z() >= z as i32
        {
            None
        } else {
            Some(pnew)
        }
    }

    fn cell_iter(&self) -> ndarray::iter::IndexedIter<'_, bool, ndarray::Ix3> {
        self.indexed_iter()
    }
}

pub struct MapSlice(Vec<Vec<i32>>);

impl MapSlice {
    fn print(&self) {
        let MapSlice(v) = self;

        for line in v {
            println!(
                "{}",
                line.iter()
                    .map(|x| x.to_string())
                    .collect::<Vec<String>>()
                    .join(" ")
            );
        }
    }

    fn max(&self) -> &i32 {
        let MapSlice(v) = self;
        v.iter().map(|x| x.iter().max().unwrap()).max().unwrap()
    }

    fn size(&self) -> Vec2i {
        let MapSlice(v) = self;
        Vec2i::new(v[0].len() as i32, v.len() as i32)
    }

    fn get(&self, p: Point2i) -> i32 {
        let MapSlice(v) = self;
        v[p.y() as usize][p.x() as usize]
    }

    fn pumped(&self) -> MapSlice {
        let MapSlice(v) = self;

        let mut out = Vec::new();
        for line in v {
            let mut new_line = Vec::new();
            for c in line {
                new_line.push(*c);
                new_line.push(*c);
            }
            out.push(new_line.clone());
            out.push(new_line);
        }
        MapSlice(out)
    }
}

#[derive(Copy, Clone, PartialEq)]
pub enum Dir {
    ZxPos,
    ZxNeg,
    YzPos,
    YzNeg,
    XyPos,
    XyNeg,
}

impl Dir {
    pub fn get_normal(&self) -> Vec3 {
        match self {
            Dir::ZxNeg => Vec3::new(0f32, -1f32, 0f32),
            Dir::ZxPos => Vec3::new(0f32, 1f32, 0f32),
            Dir::YzNeg => Vec3::new(-1f32, 0f32, 0f32),
            Dir::YzPos => Vec3::new(1f32, 0f32, 0f32),
            Dir::XyNeg => Vec3::new(0f32, 0f32, -1f32),
            Dir::XyPos => Vec3::new(0f32, 0f32, 1f32),
        }
    }

    pub fn get_normal_i(&self) -> Vec3i {
        match self {
            Dir::ZxNeg => Vec3i::new(0, -1, 0),
            Dir::ZxPos => Vec3i::new(0, 1, 0),
            Dir::YzNeg => Vec3i::new(-1, 0, 0),
            Dir::YzPos => Vec3i::new(1, 0, 0),
            Dir::XyNeg => Vec3i::new(0, 0, -1),
            Dir::XyPos => Vec3i::new(0, 0, 1),
        }
    }

    fn get_corners(&self) -> [Vec3i; NUM_PLANE_CORNERS] {
        match self {
            Dir::ZxNeg => [
                Vec3i::new(0, 0, 0),
                Vec3i::new(0, 0, 1),
                Vec3i::new(1, 0, 1),
                Vec3i::new(1, 0, 0),
            ],
            Dir::ZxPos => [
                Vec3i::new(0, 1, 0),
                Vec3i::new(1, 1, 1),
                Vec3i::new(1, 1, 1),
                Vec3i::new(0, 1, 1),
            ],

            Dir::YzNeg => [
                Vec3i::new(0, 0, 0),
                Vec3i::new(0, 1, 0),
                Vec3i::new(0, 1, 1),
                Vec3i::new(0, 0, 1),
            ],
            Dir::YzPos => [
                Vec3i::new(1, 0, 0),
                Vec3i::new(1, 0, 1),
                Vec3i::new(1, 1, 1),
                Vec3i::new(1, 1, 0),
            ],

            Dir::XyNeg => [
                Vec3i::new(0, 0, 0),
                Vec3i::new(1, 0, 0),
                Vec3i::new(1, 1, 0),
                Vec3i::new(0, 1, 0),
            ],
            Dir::XyPos => [
                Vec3i::new(0, 0, 1),
                Vec3i::new(0, 1, 1),
                Vec3i::new(1, 1, 1),
                Vec3i::new(1, 0, 1),
            ],
        }
    }
}

trait Cell {
    fn get_plane(&self, dir: Dir) -> [Point3i; 4];
}

impl Cell for Point3i {
    fn get_plane(&self, dir: Dir) -> [Point3i; 4] {
        let points = dir.get_corners();
        [
            *self + points[0],
            *self + points[1],
            *self + points[2],
            *self + points[3],
        ]
    }
}
#[derive(Clone)]
pub struct Plane {
    pub vertices: [i32; NUM_PLANE_CORNERS],
    pub dir: Dir,
    pub cell: Vec3i,
}

impl Plane {
    fn new(vertices: [i32; NUM_PLANE_CORNERS], dir: Dir, cell: Point3i) -> Plane {
        Plane {
            vertices,
            dir,
            cell,
        }
    }
}

pub struct PlanesSep {
    pub vertices: Vec<Point3i>,
    pub planes: Vec<Plane>,
}

impl PlanesSep {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            planes: Vec::new(),
        }
    }
}

impl PlanesSep {
    pub fn create_planes(&mut self, bitmap: &dyn Bitmap) {
        let mut zx_planes = Vec::<Plane>::new();
        let mut xy_planes = Vec::<Plane>::new();
        let mut yz_planes = Vec::<Plane>::new();
        let mut zxn_planes = Vec::<Plane>::new(); // need separate vecs for &mut...
        let mut xyn_planes = Vec::<Plane>::new();
        let mut yzn_planes = Vec::<Plane>::new();

        for ((x, y, z), v) in bitmap.cell_iter() {
            // println!("{} {} {}", x, y, z);
            if !v {
                continue;
            }
            let this_point = Point3i::new(x as i32, y as i32, z as i32);
            for (dir, vec) in [
                (Dir::ZxNeg, &mut zxn_planes),
                (Dir::ZxPos, &mut zx_planes),
                (Dir::XyNeg, &mut xyn_planes),
                (Dir::XyPos, &mut xy_planes),
                (Dir::YzNeg, &mut yzn_planes),
                (Dir::YzPos, &mut yz_planes),
            ]
            .iter_mut()
            {
                let create_plane = match bitmap.step(this_point, &dir) {
                    Some(p) => !Bitmap::get(bitmap, p),
                    None => *dir != Dir::ZxNeg, // don't create downfacing planes on bottom
                };

                if create_plane {
                    let local_corners: [Vec3i; NUM_PLANE_CORNERS] = dir.get_corners();
                    assert!(local_corners.len() == 4);
                    let corners = local_corners.iter().map(|x| this_point + *x);
                    let mut points = [0; NUM_PLANE_CORNERS];

                    for (i, c) in corners.enumerate() {
                        points[i] = self.vertices.len() as i32;
                        self.vertices.push(c);
                    }
                    vec.push(Plane::new(points, *dir, this_point));
                }
            }
        }

        let zx_order = |p1: &Plane, p2: &Plane| match p1.cell.y().cmp(&p2.cell.y()) {
            std::cmp::Ordering::Equal => match p1.cell.z().cmp(&p2.cell.z()) {
                std::cmp::Ordering::Equal => p1.cell.x().cmp(&p2.cell.x()),
                r => r,
            },
            r => r,
        };

        let xy_order = |p1: &Plane, p2: &Plane| match p1.cell.z().cmp(&p2.cell.z()) {
            std::cmp::Ordering::Equal => match p1.cell.x().cmp(&p2.cell.x()) {
                std::cmp::Ordering::Equal => p1.cell.y().cmp(&p2.cell.y()),
                r => r,
            },
            r => r,
        };
        let yz_order = |p1: &Plane, p2: &Plane| match p1.cell.x().cmp(&p2.cell.x()) {
            std::cmp::Ordering::Equal => match p1.cell.y().cmp(&p2.cell.y()) {
                std::cmp::Ordering::Equal => p1.cell.z().cmp(&p2.cell.z()),
                r => r,
            },
            r => r,
        };

        zx_planes.sort_by(zx_order);
        zxn_planes.sort_by(zx_order);
        xy_planes.sort_by(xy_order);
        xyn_planes.sort_by(xy_order);
        yz_planes.sort_by(yz_order);
        yzn_planes.sort_by(yz_order);

        self.planes.append(&mut zx_planes);
        self.planes.append(&mut xy_planes);
        self.planes.append(&mut yz_planes);
        self.planes.append(&mut zxn_planes);
        self.planes.append(&mut xyn_planes);
        self.planes.append(&mut yzn_planes);
    }

    #[allow(unused)]
    pub fn print(&self) {
        let mut x: Vec<(&Point3i, i32)> = self
            .vertices
            .iter()
            .enumerate()
            .map(|(i, p)| (p, i as i32))
            .collect();
        x.sort_by_key(|(_, v)| *v);

        for (k, v) in x.iter() {
            println!("{}: {}", v, DisplayWrap::from(**k));
        }

        for p in &self.planes {
            println!("{}", DisplayWrap::from(p.vertices));
        }
    }

    #[allow(unused)]
    pub fn vertex_iter(&self) -> impl Iterator<Item = (&Point3i, i32)> + '_ {
        self.vertices.iter().enumerate().map(|(i, p)| (p, i as i32))
    }

    pub fn planes_iter(&self) -> impl Iterator<Item = &Plane> + '_ {
        self.planes.iter()
    }

    pub fn num_planes(&self) -> usize {
        self.planes.len()
    }
}

pub fn to_height(c: char) -> i32 {
    match c {
        x if x >= 'a' && x <= 'z' => 1 + x as i32 - 'a' as i32,
        x if x >= '0' && x <= '9' => 2 + 'z' as i32 - 'a' as i32 + c as i32 - '0' as i32,
        _ => 0,
    }
}
pub fn read_map_slice(reader: &mut dyn std::io::BufRead, size: Vec2i) -> std::io::Result<MapSlice> {
    // let mut slice = vec![vec![width,

    let mut slice = Vec::new(); //vec![vec![0;0]];

    for _ in 0..size.y() {
        let mut line = String::new();

        reader.read_line(&mut line)?;
        let line = line.trim();

        // println!("{} {}", line.len(), width);
        assert!(line.len() == size.x() as usize);

        slice.push(line.chars().map(to_height).map(|x| x).collect());
    }
    Ok(MapSlice(slice))
}

pub fn read_map<P: AsRef<Path>>(filename: P) -> std::io::Result<Box<dyn Bitmap + Send + Sync>> {
    let file = File::open(filename)?;

    let mut reader = BufReader::new(file);

    let width;
    let height;
    {
        let mut line = String::new();
        reader.read_line(&mut line)?;

        let h: Vec<i32> = line
            .trim()
            .split_whitespace()
            .map(|x| x.parse::<i32>().unwrap())
            .collect();
        width = h[0];
        height = h[1];
    }

    let slice = read_map_slice(&mut reader, Vec2i::new(width, height))?;
    slice.print();

    // pump disabled!
    let slice = slice.pumped().pumped();
    // let slice = slice.pumped();
    let max = slice.max();
    let real_size = slice.size();

    println!("real size: {:?}", real_size);
    let mut bm = BlockMap::default((
        real_size.x() as usize,
        *max as usize,
        real_size.y() as usize,
    )); //Bitmap::new(width, *max, height);
    bm.add(&slice);
    Ok(Box::new(bm))
}

#[derive(Clone)]
pub struct PlaneScene {
    pub planes: Arc<PlanesSep>,
    pub blockmap: Arc<Box<dyn Bitmap + Send + Sync>>,
}

impl PlaneScene {
    pub fn new(planes: PlanesSep, blockmap: Box<dyn Bitmap + Send + Sync>) -> Self {
        PlaneScene {
            planes: Arc::new(planes),
            blockmap: Arc::new(blockmap),
        }
    }
}
