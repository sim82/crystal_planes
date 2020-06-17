use super::ffs::{self, Extent};
use super::{MutRadSlice, RadBuffer, RadFrontend, RadSlice};
#[allow(unused_imports)]
use crate::math::prelude::*;

use rayon::prelude::*;
use std::sync::Mutex;

#[allow(unused)]
pub struct RadBackend {
    pub emit: Vec<Vec3>,
    pub extents: Vec<Vec<ffs::Extent>>,
    pub rad_front: RadBuffer,
    pub rad_back: RadBuffer,
    pub diffuse: Vec<Vec3>,
}

#[allow(unused)]
impl RadBackend {
    pub fn new(extents: Vec<Vec<Extent>>) -> Self {
        let num_planes = extents.len();
        RadBackend {
            emit: vec![Vec3::new(0.0, 0.0, 0.0); num_planes],
            rad_front: RadBuffer::new(num_planes),
            rad_back: RadBuffer::new(num_planes),
            extents: extents,
            diffuse: vec![Vec3::new(1f32, 1f32, 1f32); num_planes],
        }
    }

    pub fn do_rad(&mut self, frontend: &Mutex<RadFrontend>) -> usize {
        self.do_rad_extents(frontend)
    }

    pub fn do_rad_extents(&mut self, frontend: &Mutex<RadFrontend>) -> usize {
        {
            let mut frontend = frontend.lock().expect("rad frontend lock failed");

            frontend.output = self.rad_back.clone();
            self.emit = frontend.emit.clone();
            self.diffuse = frontend.diffuse.clone();
        }

        std::mem::swap(&mut self.rad_front, &mut self.rad_back);
        // self.rad_front.copy

        assert!(self.rad_front.r.len() == self.extents.len());
        let mut front = RadBuffer::new(0);
        std::mem::swap(&mut self.rad_front, &mut front);

        let num_chunks = 32;
        let chunk_size = self.extents.len() / num_chunks;
        let extents_split = self.extents.chunks(chunk_size).collect::<Vec<_>>();
        let emit_split = self.emit.chunks(chunk_size).collect::<Vec<_>>();
        let diffuse_split = self.diffuse.chunks(chunk_size).collect::<Vec<_>>();

        let (r_split, g_split, b_split) = front.chunks_mut2(chunk_size);
        let mut tmp = itertools::izip!(
            r_split,
            g_split,
            b_split,
            extents_split,
            emit_split,
            diffuse_split
        )
        .collect::<Vec<_>>();

        let pint = tmp
            .par_iter_mut()
            // .iter_mut()
            .map(
                |(ref mut r, ref mut g, ref mut b, extents, emit, diffuse)| {
                    RadWorkblockScalar {
                        src: self.rad_back.slice_full(),
                        dest: (r, g, b),
                        extents,
                        emit,
                        diffuse,
                    }
                    .do_iter()
                },
            )
            .sum::<usize>();

        std::mem::swap(&mut self.rad_front, &mut front);
        pint
    }

    pub fn print_stat(&self) {}
}

struct RadWorkblockScalar<'a> {
    src: RadSlice<'a>,
    dest: MutRadSlice<'a>,
    extents: &'a [Vec<ffs::Extent>],
    emit: &'a [Vec3],
    diffuse: &'a [Vec3],
}

impl RadWorkblockScalar<'_> {
    pub fn do_iter(&mut self) -> usize {
        let mut pints: usize = 0;

        for (i, extents) in self.extents.iter().enumerate() {
            let mut rad_r = 0f32;
            let mut rad_g = 0f32;
            let mut rad_b = 0f32;
            let diffuse = self.diffuse[i as usize];

            // let RadBuffer { r, g, b } = &self.rad_back;
            let (r, g, b) = self.src;
            for ffs::Extent { start, ffs } in extents {
                for (j, ff) in ffs.iter().enumerate() {
                    unsafe {
                        rad_r += r.get_unchecked(j + *start as usize) * diffuse.x * *ff;
                        rad_g += g.get_unchecked(j + *start as usize) * diffuse.y * *ff;
                        rad_b += b.get_unchecked(j + *start as usize) * diffuse.z * *ff;
                    }
                }
                pints += ffs.len();
            }

            self.dest.0[i as usize] = self.emit[i as usize].x + rad_r;
            self.dest.1[i as usize] = self.emit[i as usize].y + rad_g;
            self.dest.2[i as usize] = self.emit[i as usize].z + rad_b;
        }

        pints
    }
}
