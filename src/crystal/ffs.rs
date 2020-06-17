use super::{util, BlockMap, Dir, Plane, PlanesSep};
use crate::math::prelude::*;
use amethyst::core::math;
use image::ImageBuffer;
use std::{
    cmp::Ordering,
    fmt::Debug,
    io::{BufReader, BufWriter},
};
fn normal_cull(pl1: &Plane, pl2: &Plane) -> bool {
    let d1 = pl1.dir;
    let d2 = pl2.dir;

    let p1 = pl1.cell;
    let p2 = pl2.cell;

    p1 == p2
        || d1 == d2
        || (d1 == Dir::XyNeg && d2 == Dir::XyPos && p1.z < p2.z)
        || (d1 == Dir::XyPos && d2 == Dir::XyNeg && p1.z > p2.z)
        || (d1 == Dir::YzNeg && d2 == Dir::YzPos && p1.x < p2.x)
        || (d1 == Dir::YzPos && d2 == Dir::YzNeg && p1.x > p2.x)
        || (d1 == Dir::ZxNeg && d2 == Dir::ZxPos && p1.y < p2.y)
        || (d1 == Dir::ZxPos && d2 == Dir::ZxNeg && p1.y > p2.y)
}

fn setup_formfactors_single(planes: &PlanesSep, bitmap: &BlockMap) -> Vec<(u32, u32, f32)> {
    let planes = planes.planes_iter().collect::<Vec<&Plane>>();
    println!("num planes: {}", planes.len());
    let mut ffs = Vec::new();
    for (i, plane1) in planes.iter().enumerate() {
        let norm1 = plane1.dir.get_normal::<i32>();
        let norm1f = Vec3::new(norm1.x as f32, norm1.y as f32, norm1.z as f32);
        let p1f = Vec3::new(
            plane1.cell.x as f32,
            plane1.cell.y as f32,
            plane1.cell.z as f32,
        );
        // println!("{}", i);
        for j in 0..i {
            let plane2 = planes[j];
            let norm2 = plane2.dir.get_normal::<i32>();
            let norm2f = Vec3::new(norm2.x as f32, norm2.y as f32, norm2.z as f32);
            let p2f = Vec3::new(
                plane2.cell.x as f32,
                plane2.cell.y as f32,
                plane2.cell.z as f32,
            );
            if normal_cull(plane1, plane2) {
                // println!("normal_cull");
                continue;
            }

            let dn = (p1f - p2f).normalize();
            let d2 = (p1f - p2f).magnitude_squared(); // uhm, will the compiler optimize the two calls?

            let ff1 = 0.0f32.max(math::Matrix::dot(&norm1f, &(Vec3::new(0.0, 0.0, 0.0) - dn)));
            let ff2 = 0.0f32.max(math::Matrix::dot(&norm2f, &dn));

            let ff = (ff1 * ff2) / (3.1415 * d2);
            let dist_cull = ff < 5e-6;

            if !dist_cull && !util::occluded(plane1.cell + norm1, plane2.cell + norm2, bitmap) {
                ffs.push((i as u32, j as u32, ff));
            }
        }
    }

    ffs
}

pub fn setup_formfactors(planes: &PlanesSep, bitmap: &BlockMap) -> Vec<(u32, u32, f32)> {
    let mut ffs = setup_formfactors_single(planes, bitmap);

    println!("num ffs: {}", ffs.len());

    let mut ffs2 = ffs.iter().map(|(i, j, ff)| (*j, *i, *ff)).collect();

    ffs.append(&mut ffs2);

    ffs.sort_unstable_by(
        |l: &(u32, u32, f32), r: &(u32, u32, f32)| match l.0.cmp(&r.0) {
            Ordering::Equal => l.1.cmp(&r.1),
            Ordering::Less => Ordering::Less,
            Ordering::Greater => Ordering::Greater,
        },
    );
    println!("sorted");
    ffs
}

#[allow(dead_code)]
fn write_ffs_debug(ffs: &Vec<(u32, u32, f32)>) {
    let width = ffs.iter().map(|(x, _, _)| *x).max().unwrap_or(0) + 1;
    let height = ffs.iter().map(|(_, y, _)| *y).max().unwrap_or(0) + 1;
    let maxf = ffs
        .iter()
        .map(|(_, _, f)| *f)
        .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)) // seriously?
        .unwrap_or(0f32);

    println!("{} {} {}", width, height, maxf);
    println!("painting...");
    let mut image = ImageBuffer::new(width, height);

    for (x, y, _) in ffs {
        let pixel = image.get_pixel_mut(*x, *y);
        *pixel = image::Luma([255u8]);
    }
    println!("writing ffs.png");

    image.save("ffs.png").unwrap();
    println!("done");
}

pub fn split_formfactors(ff_in: Vec<(u32, u32, f32)>) -> Vec<Vec<(u32, f32)>> {
    let num = ff_in.iter().map(|(i, _, _)| i).max().unwrap() + 1;

    let mut ff_out = vec![Vec::new(); num as usize];
    for (i, j, ff) in ff_in.iter() {
        ff_out[*i as usize].push((*j, *ff));
    }

    ff_out
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Extent {
    pub start: u32,
    pub ffs: Vec<f32>,
}

impl Extent {
    fn new(start: u32, ffs: Vec<f32>) -> Self {
        Extent {
            start: start,
            ffs: ffs,
        }
    }

    pub fn split_aligned(&self, alignments: &[usize]) -> Vec<Extent> {
        let first = self.start as usize;
        let mut i = first;
        let end = first + self.ffs.len();
        let mut out = Vec::new();

        while i < end {
            for a in alignments {
                if i % *a == 0 && end - i >= *a {
                    out.push(Extent::new(
                        i as u32,
                        self.ffs[i - first..i + a - first].to_vec(),
                    ));
                    i += a;
                    break;
                }
            }
        }
        out
    }
}
impl Debug for Extent {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "ext: {} +{}", self.start, self.ffs.len())
    }
}

fn to_extent(v: &Vec<(u32, f32)>) -> Vec<Extent> {
    let mut cur_ext: Option<(u32, Extent)> = None;
    let mut extents = Vec::new();

    for (pos, ff) in v.iter() {
        if let Some((last, mut ext)) = cur_ext {
            if *pos == last + 1 {
                ext.ffs.push(*ff);
                cur_ext = Some((*pos, ext));
            } else {
                extents.push(ext);
                cur_ext = Some((*pos, Extent::new(*pos as u32, vec![*ff])));
            }
        } else {
            cur_ext = Some((*pos, Extent::new(*pos as u32, vec![*ff])));
        }
    }

    if let Some((_, ext)) = cur_ext {
        extents.push(ext);
    }
    extents
}

pub fn to_extents(ffs: &Vec<Vec<(u32, f32)>>) -> Vec<Vec<Extent>> {
    ffs.iter().map(|v| to_extent(v)).collect()
}
const EXTENT_VERSION: &str = "extents v1";

pub fn load_extents(filename: &str) -> Option<Vec<Vec<Extent>>> {
    if let Ok(f) = std::fs::File::open(filename) {
        println!("read from {}", filename);
        let (file_version, extents): (String, Vec<Vec<Extent>>) =
            bincode::deserialize_from(BufReader::new(f)).unwrap();

        if file_version == EXTENT_VERSION {
            println!("done");
            return Some(extents);
        }
        println!("wrong version");
    }
    return None;
}

pub fn write_extents(filename: &str, extents: &Vec<Vec<Extent>>) {
    if let Ok(file) = std::fs::File::create(filename) {
        bincode::serialize_into(BufWriter::new(file), &(&EXTENT_VERSION, extents)).unwrap();
    }
    println!("wrote {}", filename);
}
