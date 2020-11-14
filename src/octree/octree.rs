use std::fmt::Debug;

use crate::math::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct OctantId(u32);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Voxel {
    Empty,
    Leaf,
    Octant(OctantId),
}

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct IdxPath(usize);
impl Debug for IdxPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:o}", self.0)
    }
    // fn fmt()
}

// pub enum Voxel {
//     Leaf,
//     Empty,
//     Oxtant(OctantId)
// }

#[derive(Debug)]
pub struct Octant {
    pub children: [Voxel; 8],
    pub scale: u32,
    idx_path: IdxPath,
    id: OctantId,
}

pub struct Octants {
    pub octants: Vec<Octant>,
}

impl Default for Octants {
    fn default() -> Octants {
        Octants {
            octants: Vec::new(),
        }
    }
}

impl Octants {
    pub fn new(&mut self, scale: u32, coord: IdxPath) -> OctantId {
        assert!(scale >= 1);
        let id = OctantId(self.octants.len() as u32);
        self.octants.push(Octant {
            children: [Voxel::Empty; 8],
            scale,
            idx_path: coord,
            id,
        });
        id
    }
    pub fn get(&self, id: OctantId) -> &Octant {
        &self.octants[id.0 as usize]
    }
    pub fn get_mut(&mut self, id: OctantId) -> &mut Octant {
        &mut self.octants[id.0 as usize]
    }
    pub fn get_id_iter(&self) -> impl Iterator<Item = OctantId> {
        (0..self.octants.len()).map(|i| OctantId(i as u32))
    }
}

pub fn scale1_voxels(octants: &mut Octants, points: &[Point3i]) -> Vec<OctantId> {
    let mut zordered: Vec<usize> = points.iter().map(|p| zorder(p)).collect();
    zordered.sort_unstable();
    let mut coord_to_octant = std::collections::HashMap::new();

    for z in zordered {
        let coord = IdxPath(z >> 3);

        let id = coord_to_octant
            .entry(coord)
            .or_insert_with(|| octants.new(1, coord));

        let level_coord = z & 0b111;
        let octant = octants.get_mut(*id);
        octant.children[level_coord] = Voxel::Leaf;
    }
    coord_to_octant.drain().map(|(_, v)| v).collect()
}

pub fn create_octants_bottom_up(octantss: &mut Octants, points: &[Point3i]) -> Option<OctantId> {
    let mut octants_last = scale1_voxels(octantss, points);
    // let mut octants = Vec::new();
    for level in 2.. {
        if octants_last.len() == 1 {
            break;
        }
        let mut octants_out = std::collections::HashMap::new();

        for id in octants_last.iter() {
            let octant = octantss.get(*id);
            let coord = IdxPath(octant.idx_path.0 >> 3);
            let level_coord = octant.idx_path.0 & 0b111;

            let octant_out = octants_out
                .entry(coord)
                .or_insert_with(|| octantss.new(level, coord));
            octantss.get_mut(*octant_out).children[level_coord] = Voxel::Octant(*id);
            // octant_out.children[level_coord] = Child::Octant(oc)
        }
        octants_last = octants_out.drain().map(|(_, v)| v).collect();
    }
    octants_last.get(0).cloned()
}

pub fn child_offset(i: usize) -> Vec3i {
    match i {
        0 => Point3i::new(0, 0, 0),
        1 => Point3i::new(1, 0, 0),
        2 => Point3i::new(0, 1, 0),
        3 => Point3i::new(1, 1, 0),
        4 => Point3i::new(0, 0, 1),
        5 => Point3i::new(1, 0, 1),
        6 => Point3i::new(0, 1, 1),
        7 => Point3i::new(1, 1, 1),
        _ => panic!("bad child index"),
    }
}

#[allow(dead_code)]
pub fn generate_points(octants: &Octants, root: OctantId, offset: &Point3i) -> Vec<Point3i> {
    let octant = octants.get(root);
    let mut out = Vec::new();
    // println!("{:?}", octant);

    for (i, child) in octant.children.iter().enumerate() {
        let child_offs = child_offset(i);
        if let Voxel::Leaf = child {
            out.push(child_offs + *offset * 2);
        // println!("{:?}", child_offs + *offset * 2);
        } else if let Voxel::Octant(id) = child {
            out.append(&mut generate_points(
                octants,
                *id,
                &(*offset * 2 + child_offs),
            ));
        }
    }
    out
}

pub fn zorder(p: &Point3i) -> usize {
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

#[test]
fn scale1_test() {
    {
        let mut octants = Octants::default();
        let points = [
            Point3i::new(0, 0, 0),
            Point3i::new(1, 0, 0),
            Point3i::new(0, 1, 0),
            Point3i::new(0, 0, 1),
            Point3i::new(1, 1, 0),
            Point3i::new(0, 1, 1),
            Point3i::new(1, 0, 1),
            Point3i::new(1, 1, 1),
        ];

        let l0 = scale1_voxels(&mut octants, &points);
        assert_eq!(l0.len(), 1);
        assert_eq!(octants.get(l0[0]).children, [Voxel::Leaf; 8]);
    }
    {
        let mut octants = Octants::default();
        let l0 = scale1_voxels(&mut octants, &[Point3i::new(0, 0, 0)]);
        assert_eq!(
            octants.get(l0[0]).children,
            [
                Voxel::Leaf,
                Voxel::Empty,
                Voxel::Empty,
                Voxel::Empty,
                Voxel::Empty,
                Voxel::Empty,
                Voxel::Empty,
                Voxel::Empty
            ]
        )
    }
}

#[test]
fn test_bottom_up() {
    let mut octants = Octants::default();
    let points = [
        Point3i::new(0, 0, 0),
        Point3i::new(1, 0, 0),
        Point3i::new(0, 1, 0),
        Point3i::new(0, 0, 1),
        Point3i::new(1, 1, 0),
        Point3i::new(0, 1, 1),
        Point3i::new(1, 0, 1),
        Point3i::new(1, 1, 1),
        Point3i::new(2, 2, 2),
        Point3i::new(3, 2, 2),
        Point3i::new(2, 3, 2),
        Point3i::new(2, 2, 3),
        Point3i::new(3, 3, 2),
        Point3i::new(2, 3, 3),
        Point3i::new(3, 2, 3),
        Point3i::new(3, 3, 3),
    ];
    let id = create_octants_bottom_up(&mut octants, &points);
    assert!(id.is_some());
    if let Some(id) = id {
        println!("{:?}", octants.get(id));
        let octant = octants.get(id);
        if let [Voxel::Octant(id0), Voxel::Empty, Voxel::Empty, Voxel::Empty, Voxel::Empty, Voxel::Empty, Voxel::Empty, Voxel::Octant(id7)] =
            octant.children
        {
            let octant0 = octants.get(id0);
            let octant7 = octants.get(id7);
            println!("{:?}", octant0);
            println!("{:?}", octant7);
        }
    }
}

#[test]
fn test_bottom_up2() {
    let mut octants = Octants::default();
    let points = [
        Point3i::new(0, 0, 0),
        // Point3i::new(1, 0, 0),
        // Point3i::new(0, 1, 0),
        // Point3i::new(0, 0, 1),
        // Point3i::new(1, 1, 0),
        // Point3i::new(0, 1, 1),
        // Point3i::new(1, 0, 1),
        // Point3i::new(1, 1, 1),
        // Point3i::new(2, 0, 0),
        // Point3i::new(4, 0, 0),
        // Point3i::new(6, 1, 0),
        // Point3i::new(8, 0, 1),
        // Point3i::new(10, 1, 0),
        // Point3i::new(12, 1, 1),
        Point3i::new(14, 14, 14),
        Point3i::new(15, 14, 14),
        Point3i::new(14, 15, 14),
        Point3i::new(14, 14, 15),
        Point3i::new(15, 15, 14),
        Point3i::new(14, 15, 15),
        Point3i::new(15, 14, 15),
        Point3i::new(15, 15, 15),
    ];
    let id = create_octants_bottom_up(&mut octants, &points);
    assert!(id.is_some());
    // let id = id.unwrap();
    for octant in octants.octants.iter() {
        for c in octant.children.iter() {
            match c {
                Voxel::Leaf => assert_eq!(octant.scale, 1), // only scale 1 octants have leafs
                Voxel::Octant(child_id) => {
                    // make sure that octants of scale are only referenced by octants of scale + 1
                    assert_eq!(octants.get(*child_id).scale + 1, octant.scale)
                }
                _ => (),
            }
        }
        println!("{:?}", octant);
    }

    let points_gen = generate_points(&octants, id.unwrap(), &Vec3i::zero());
    println!("points: {:?}", points_gen);

    assert_eq!(points.len(), points_gen.len());

    for gen in points_gen.iter() {
        assert!(points.iter().find(|x| **x == *gen).is_some());
    }

    // println!("{:?}", octants.get(id));
    // let octant = octants.get(id);
    // if let [Child::Octant(id0), Child::Empty, Child::Empty, Child::Empty, Child::Empty, Child::Empty, Child::Empty, Child::Octant(id7)] =
    //     octant.children
    // {
    //     let octant0 = octants.get(id0);
    //     let octant7 = octants.get(id7);
    //     println!("{:?}", octant0);
    //     println!("{:?}", octant7);
    // }
}

impl Octant {
    pub fn get_geometry(&self, height: u32) -> (Vec3i, Vec3i) {
        let mut pos = Vec3i::zero();

        let mut coord = self.idx_path.0;
        let mut size_mul = 2;
        for _ in self.scale..height {
            pos += child_offset(coord & 0b111) * size_mul;
            size_mul *= 2;
            coord >>= 3;
        }
        let mut size = 2;
        for _ in 1..self.scale {
            pos *= 2;
            size *= 2;
        }

        (pos, Vec3i::one() * size)
    }
}

#[test]
fn test_get_geometry() {
    let mut octants = Octants::default();
    let points = [
        Point3i::new(0, 0, 0),
        Point3i::new(2, 0, 0),
        Point3i::new(4, 0, 0),
        Point3i::new(6, 1, 0),
        Point3i::new(8, 0, 1),
        Point3i::new(10, 1, 0),
        Point3i::new(12, 1, 1),
    ];
    let id = create_octants_bottom_up(&mut octants, &points);
    assert!(id.is_some());
    // let id = id.unwrap();

    // for octant in octants.octants.iter() {
    //     println!("{:?}", octant);
    // }

    let mut generated_leaf_set = std::collections::HashSet::new();
    for octant in octants.octants.iter() {
        let (pos, size) = octant.get_geometry(4);
        println!("{:?}", octant);
        println!("octant: {:?} {:?}", pos, size);
        assert_eq!(size.0, 2i32.pow(octant.scale));

        for (i, child) in octant.children.iter().enumerate() {
            if *child == Voxel::Leaf {
                let offs = child_offset(i);
                println!("leaf: {:?}", pos + offs);
                generated_leaf_set.insert(pos + offs);
            }
        }
    }

    assert_eq!(generated_leaf_set, points.iter().cloned().collect());
}
