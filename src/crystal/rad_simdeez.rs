#[allow(unused_imports)]
use super::{
    aligned_vector_init, Bitmap, BlockMap, DisplayWrap, MutRadSlice, RadBuffer, RadFrontend,
    RadSlice,
};
use super::{ffs, ffs::Extent};
use crate::math::prelude::*;

use rayon::prelude::*;
use simdeez::{avx2::Avx2, sse2::Sse2, Simd};
use std::sync::Mutex;

pub struct Blocklist {
    single: Vec<(u32, f32)>,
    vec4_ff: Vec<<Sse2 as Simd>::Vf32>,
    vec8_ff: Vec<<Avx2 as Simd>::Vf32>,
    vec2: Vec<u32>,
    vec4: Vec<u32>,
    vec8: Vec<u32>,
    vec16: Vec<u32>,
}

impl Blocklist {
    pub fn from_extents(extents: &Vec<ffs::Extent>) -> Blocklist {
        let vec16 = Vec::new();
        let mut vec8 = Vec::new();
        let mut vec4 = Vec::new();
        let vec2 = Vec::new();
        // let mut vec16_ff = Vec::new();
        let mut vec8_ff = Vec::new();
        let mut vec4_ff = Vec::new();
        // let mut vec2_ff = Vec::new();
        let mut single = Vec::new();

        for ext in extents.iter().flat_map(|x| x.split_aligned(&[8, 4, 1])) {
            match ext.ffs.len() {
                8 => {
                    vec8.push(ext.start);
                    unsafe {
                        vec8_ff.push(Avx2::loadu_ps(&ext.ffs[0]));
                    }
                }
                4 => {
                    vec4.push(ext.start);
                    unsafe {
                        vec4_ff.push(Sse2::loadu_ps(&ext.ffs[0]));
                    }
                }
                1 => single.push((ext.start, ext.ffs[0])),
                _ => panic!("bad extent size: {}", ext.ffs.len()),
            }
        }

        Blocklist {
            single: single,
            vec2: vec2,
            vec4: vec4,
            vec8: vec8,
            vec16: vec16,
            vec4_ff: vec4_ff,
            vec8_ff: vec8_ff,
        }
    }
    #[allow(unused)]
    pub fn print_stat(&self) {
        println!(
            "1: {} 4: {} 8: {}",
            self.single.len(),
            self.vec4.len(),
            self.vec8.len(),
        );
    }
    #[allow(unused)]
    pub fn num_formfactors(&self) -> usize {
        return self.single.len()
            + self.vec2.len() * 2
            + self.vec4.len() * 4
            + self.vec8.len() * 8
            + self.vec16.len() * 16;
    }

    pub fn get_sizes(&self) -> (usize, usize, usize) {
        return (self.single.len(), self.vec4.len(), self.vec8.len());
    }
}

pub struct RadBackend {
    pub emit: Vec<Vec3>,
    pub blocks: Vec<Blocklist>,
    pub extents: Vec<Vec<ffs::Extent>>,
    pub rad_front: RadBuffer,
    pub rad_back: RadBuffer,
    pub diffuse: Vec<Vec3>,
}

impl RadBackend {
    pub fn new(extents: Vec<Vec<Extent>>) -> Self {
        let blocks = extents
            .iter()
            .map(|x| Blocklist::from_extents(x))
            .collect::<Vec<_>>();
        let sizes = blocks
            .iter()
            .map(|x| x.get_sizes())
            .fold((0, 0, 0), |(acc_a, acc_b, acc_c), (a, b, c)| {
                (acc_a + a, acc_b + b * 4, acc_c + c * 8)
            });
        let size_all = (sizes.0 + sizes.1 + sizes.2) as f32;
        println!(
            "sizes: 1: {} {:.4} 4: {} {:.4} 8: {} {:.4}",
            sizes.0,
            sizes.0 as f32 / size_all,
            sizes.1,
            sizes.1 as f32 / size_all,
            sizes.2,
            sizes.2 as f32 / size_all
        );

        let num_planes = extents.len();
        RadBackend {
            emit: vec![Vec3::new(0.0, 0.0, 0.0); num_planes],
            // rad_front: vec![Vec3::zero(); planes.num_planes()],
            // rad_back: vec![Vec3::zero(); planes.num_planes()],
            rad_front: RadBuffer::new(num_planes),
            rad_back: RadBuffer::new(num_planes),
            blocks: blocks,
            extents: extents,
            //ff: formfactors,
            diffuse: vec![Vec3::new(1f32, 1f32, 1f32); num_planes],
        }
    }

    pub fn do_rad(&mut self, frontend: &Mutex<RadFrontend>) -> usize {
        self.do_rad_blocks(frontend)
    }

    pub fn do_rad_blocks(&mut self, frontend: &Mutex<RadFrontend>) -> usize {
        {
            let mut frontend = frontend.lock().expect("rad frontend lock failed");

            frontend.output = self.rad_back.clone();
            self.emit = frontend.emit.clone();
            self.diffuse = frontend.diffuse.clone();
        }
        std::mem::swap(&mut self.rad_front, &mut self.rad_back);

        assert!(self.rad_front.r.len() == self.blocks.len());
        let mut front = RadBuffer::new(0);
        std::mem::swap(&mut self.rad_front, &mut front);

        let num_chunks = 32;
        let chunk_size = self.blocks.len() / num_chunks;
        let blocks_split = self.blocks.chunks(chunk_size).collect::<Vec<_>>();
        let emit_split = self.emit.chunks(chunk_size).collect::<Vec<_>>();
        let diffuse_split = self.diffuse.chunks(chunk_size).collect::<Vec<_>>();

        let (r_split, g_split, b_split) = front.chunks_mut2(chunk_size);
        let mut tmp = itertools::izip!(
            // front.chunks_mut(chunk_size),
            r_split,
            g_split,
            b_split,
            blocks_split,
            emit_split,
            diffuse_split
        )
        .collect::<Vec<_>>();

        let pints = tmp
            .par_iter_mut()
            // .iter_mut()
            .map(|(ref mut r, ref mut g, ref mut b, blocks, emit, diffuse)| {
                RadWorkblockSimd::new(self.rad_back.slice_full(), (r, g, b), blocks, emit, diffuse)
                    .do_iter()
            })
            .sum::<usize>();

        std::mem::swap(&mut self.rad_front, &mut front);

        pints
    }
    #[allow(unused)]
    pub fn print_stat(&self) {
        // let internal = self.internal.lock().expect("rad internal lock failed");
        // println!("write blocks");

        // for blocklist in &self.blocks {
        //     blocklist.print_stat();
        // }

        let ff_size: usize = self.blocks.iter().map(|x| x.num_formfactors() * 4).sum();
        let color_size = self.rad_front.r.len() * 3 * 4 * 2;

        println!("working set:\nff: {}\ncolor: {}", ff_size, color_size);
    }
}

struct RadWorkblockSimd<'a> {
    src: RadSlice<'a>,
    dest: MutRadSlice<'a>,
    blocks: &'a [Blocklist],
    emit: &'a [Vec3],
    diffuse: &'a [Vec3],
    vtmp: Vec<f32>,
}

impl RadWorkblockSimd<'_> {
    pub fn new<'a>(
        src: RadSlice<'a>,
        dest: MutRadSlice<'a>,
        blocks: &'a [Blocklist],
        emit: &'a [Vec3],
        diffuse: &'a [Vec3],
    ) -> RadWorkblockSimd<'a> {
        RadWorkblockSimd {
            src: src,
            dest: dest,
            blocks: blocks,
            emit: emit,
            diffuse: diffuse,
            vtmp: aligned_vector_init(16, 64, 0.0),
        }
    }
    pub fn do_iter(&mut self) -> usize {
        let mut pints: usize = 0;
        for (i, ff_i) in self.blocks.iter().enumerate() {
            // let mut rad = Vec3::zero();

            let mut rad_r = 0f32;
            let mut rad_g = 0f32;
            let mut rad_b = 0f32;
            let diffuse = self.diffuse[i as usize];

            let (r, g, b) = self.src;
            for (j, ff) in &ff_i.single {
                unsafe {
                    rad_r += r.get_unchecked(*j as usize) * diffuse.x * *ff;
                    rad_g += g.get_unchecked(*j as usize) * diffuse.y * *ff;
                    rad_b += b.get_unchecked(*j as usize) * diffuse.z * *ff;
                }
            }

            unsafe {
                type V = Sse2;

                let vdiffuse_r = V::set1_ps(diffuse.x);
                let vdiffuse_g = V::set1_ps(diffuse.y);
                let vdiffuse_b = V::set1_ps(diffuse.z);

                let mut vsum_r = V::setzero_ps();
                let mut vsum_g = V::setzero_ps();
                let mut vsum_b = V::setzero_ps();

                for (j, ff) in ff_i.vec4.iter().zip(&ff_i.vec4_ff) {
                    // unsafe {
                    let j = *j as usize;
                    let vr = V::load_ps(&r.get_unchecked(j));
                    let vg = V::load_ps(&g.get_unchecked(j));
                    let vb = V::load_ps(&b.get_unchecked(j));

                    vsum_r += vdiffuse_r * *ff * vr;
                    vsum_g += vdiffuse_g * *ff * vg;
                    vsum_b += vdiffuse_b * *ff * vb;
                }
                V::store_ps(&mut self.vtmp[0], vsum_r);
                rad_r += self.vtmp.iter().take(V::VF32_WIDTH).sum::<f32>();
                V::store_ps(&mut self.vtmp[0], vsum_g);
                rad_g += self.vtmp.iter().take(V::VF32_WIDTH).sum::<f32>();
                V::store_ps(&mut self.vtmp[0], vsum_b);
                rad_b += self.vtmp.iter().take(V::VF32_WIDTH).sum::<f32>();
            }

            unsafe {
                type V = Avx2;

                let vdiffuse_r = V::set1_ps(diffuse.x);
                let vdiffuse_g = V::set1_ps(diffuse.y);
                let vdiffuse_b = V::set1_ps(diffuse.z);

                let mut vsum_r = V::setzero_ps();
                let mut vsum_g = V::setzero_ps();
                let mut vsum_b = V::setzero_ps();

                for (j, ff) in ff_i.vec8.iter().zip(&ff_i.vec8_ff) {
                    // unsafe {
                    let j = *j as usize;
                    let vr = V::load_ps(&r.get_unchecked(j));
                    let vg = V::load_ps(&g.get_unchecked(j));
                    let vb = V::load_ps(&b.get_unchecked(j));

                    vsum_r += vdiffuse_r * *ff * vr;
                    vsum_g += vdiffuse_g * *ff * vg;
                    vsum_b += vdiffuse_b * *ff * vb;
                }
                V::store_ps(&mut self.vtmp[0], vsum_r);
                rad_r += self.vtmp.iter().take(V::VF32_WIDTH).sum::<f32>();
                V::store_ps(&mut self.vtmp[0], vsum_g);
                rad_g += self.vtmp.iter().take(V::VF32_WIDTH).sum::<f32>();
                V::store_ps(&mut self.vtmp[0], vsum_b);
                rad_b += self.vtmp.iter().take(V::VF32_WIDTH).sum::<f32>();
            }
            self.dest.0[i as usize] = self.emit[i as usize].x + rad_r;
            self.dest.1[i as usize] = self.emit[i as usize].y + rad_g;
            self.dest.2[i as usize] = self.emit[i as usize].z + rad_b;

            pints += ff_i.single.len()
                + ff_i.vec2.len() * 2
                + ff_i.vec4.len() * 4
                + ff_i.vec8.len() * 8
                + ff_i.vec16.len() * 16;
        }
        pints
    }
}
