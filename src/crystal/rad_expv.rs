use super::{ffs, util, PlanesSep};
#[allow(unused_imports)]
use super::{Bitmap, BlockMap, DisplayWrap, Point3, Point3i, Vec3, Vec3i};
use packed_simd::{f32x16, f32x2, f32x4, f32x8};
use rayon::prelude::*;
use std::time::Instant;

pub struct RadBuffer {
    pub r: Vec<f32>,
    pub g: Vec<f32>,
    pub b: Vec<f32>,
}
type RadSlice<'a> = (&'a [f32], &'a [f32], &'a [f32]);
type MutRadSlice<'a> = (&'a mut [f32], &'a mut [f32], &'a mut [f32]);

impl RadBuffer {
    /// Utility for making specifically aligned vectors
    pub fn aligned_vector<T>(len: usize, align: usize) -> Vec<T> {
        let t_size = std::mem::size_of::<T>();
        let t_align = std::mem::align_of::<T>();
        let layout = if t_align >= align {
            std::alloc::Layout::from_size_align(t_size * len, t_align).unwrap()
        } else {
            std::alloc::Layout::from_size_align(t_size * len, align).unwrap()
        };
        unsafe {
            let mem = std::alloc::alloc(layout);
            assert_eq!((mem as usize) % 16, 0);
            Vec::<T>::from_raw_parts(mem as *mut T, len, len)
        }
    }

    pub fn aligned_vector_init<T: Copy>(len: usize, align: usize, init: T) -> Vec<T> {
        let mut v = Self::aligned_vector::<T>(len, align);
        for x in v.iter_mut() {
            *x = init;
        }
        v
    }

    fn new(size: usize) -> RadBuffer {
        RadBuffer {
            r: Self::aligned_vector_init(size, 64, 0f32),
            g: Self::aligned_vector_init(size, 64, 0f32),
            b: Self::aligned_vector_init(size, 64, 0f32),
        }
    }

    pub fn slice(&self, i: std::ops::Range<usize>) -> RadSlice<'_> {
        (&self.r[i.clone()], &self.g[i.clone()], &self.b[i.clone()])
    }
    pub fn slice_mut(&mut self, i: std::ops::Range<usize>) -> MutRadSlice<'_> {
        (
            &mut self.r[i.clone()],
            &mut self.g[i.clone()],
            &mut self.b[i.clone()],
        )
    }
    // this is a bit redundant, but found no better way since SliceIndex is non-copy and thus cannot be used for indexing multiple Vecs
    pub fn slice_full(&self) -> RadSlice<'_> {
        (&self.r[..], &self.g[..], &self.b[..])
    }
    pub fn slice_full_mut(&mut self) -> MutRadSlice<'_> {
        (&mut self.r[..], &mut self.g[..], &mut self.b[..])
    }

    pub fn chunks_mut(&mut self, size: usize) -> impl Iterator<Item = MutRadSlice<'_>> {
        itertools::izip!(
            self.r.chunks_mut(size),
            self.g.chunks_mut(size),
            self.b.chunks_mut(size)
        )
    }

    fn chunks_mut2(
        &mut self,
        size: usize,
    ) -> (
        impl Iterator<Item = &mut [f32]>,
        impl Iterator<Item = &mut [f32]>,
        impl Iterator<Item = &mut [f32]>,
    ) {
        (
            self.r.chunks_mut(size),
            self.g.chunks_mut(size),
            self.b.chunks_mut(size),
        )
    }
}

pub struct Blocklist {
    single: Vec<(u32, f32)>,
    vec2_ff: Vec<f32x2>, // keep simd vectors densely packed
    vec4_ff: Vec<f32x4>,
    vec8_ff: Vec<f32x8>,
    vec16_ff: Vec<f32x16>,
    vec2: Vec<u32>,
    vec4: Vec<u32>,
    vec8: Vec<u32>,
    vec16: Vec<u32>,
}

impl Blocklist {
    pub fn from_extents(extents: &Vec<ffs::Extent>) -> Blocklist {
        let mut vec16 = Vec::new();
        let mut vec8 = Vec::new();
        let mut vec4 = Vec::new();
        let mut vec2 = Vec::new();
        let mut vec16_ff = Vec::new();
        let mut vec8_ff = Vec::new();
        let mut vec4_ff = Vec::new();
        let mut vec2_ff = Vec::new();
        let mut single = Vec::new();

        for ext in extents
            .iter()
            .flat_map(|x| x.split_aligned(&[16, 8, 4, 2, 1]))
        {
            match ext.ffs.len() {
                16 => {
                    vec16.push(ext.start);
                    vec16_ff.push(f32x16::from_slice_unaligned(&ext.ffs[..]));
                }
                8 => {
                    vec8.push(ext.start);
                    vec8_ff.push(f32x8::from_slice_unaligned(&ext.ffs[..]));
                }
                4 => {
                    vec4.push(ext.start);
                    vec4_ff.push(f32x4::from_slice_unaligned(&ext.ffs[..]));
                }
                2 => {
                    vec2.push(ext.start);
                    vec2_ff.push(f32x2::from_slice_unaligned(&ext.ffs[..]));
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
            vec2_ff: vec2_ff,
            vec4_ff: vec4_ff,
            vec8_ff: vec8_ff,
            vec16_ff: vec16_ff,
        }
    }

    pub fn print_stat(&self) {
        println!(
            "1: {} 2: {} 4: {} 8: {}",
            self.single.len(),
            self.vec2.len(),
            self.vec4.len(),
            self.vec8.len(),
        );
    }

    pub fn num_formfactors(&self) -> usize {
        return self.single.len()
            + self.vec2.len() * 2
            + self.vec4.len() * 4
            + self.vec8.len() * 8
            + self.vec16.len() * 16;
    }
}

pub struct Scene {
    pub planes: PlanesSep,
    pub bitmap: BlockMap,
    pub emit: Vec<Vec3>,
    pub blocks: Vec<Blocklist>,
    pub extents: Vec<Vec<ffs::Extent>>,
    pub rad_front: RadBuffer,
    pub rad_back: RadBuffer,
    pub diffuse: Vec<Vec3>,
    pub pints: usize,
}

fn vec_mul(v1: &Vec3, v2: &Vec3) -> Vec3 {
    Vec3::new(v1.x * v2.x, v1.y * v2.y, v1.z * v2.z)
}

impl Scene {
    pub fn new(planes: PlanesSep, bitmap: BlockMap) -> Self {
        let filename = "extents.bin";

        let extents = if let Some(extents) = ffs::load_extents(filename) {
            extents
        } else {
            let formfactors = ffs::split_formfactors(ffs::setup_formfactors(&planes, &bitmap));
            let extents = ffs::to_extents(&formfactors);
            ffs::write_extents(filename, &extents);
            println!("wrote {}", filename);
            extents
        };

        let start = Instant::now();
        let blocks = extents
            .iter()
            .map(|x| Blocklist::from_extents(x))
            .collect::<Vec<_>>();
        println!("blocks done: {:?}", start.elapsed());

        Scene {
            emit: vec![Vec3::new(0.0, 0.0, 0.0); planes.num_planes()],
            // rad_front: vec![Vec3::zero(); planes.num_planes()],
            // rad_back: vec![Vec3::zero(); planes.num_planes()],
            rad_front: RadBuffer::new(planes.num_planes()),
            rad_back: RadBuffer::new(planes.num_planes()),
            blocks: blocks,
            extents: extents,
            //ff: formfactors,
            diffuse: vec![Vec3::new(1f32, 1f32, 1f32); planes.num_planes()],
            planes: planes,
            bitmap: bitmap,
            pints: 0,
        }
    }

    pub fn clear_emit(&mut self) {
        for v in self.emit.iter_mut() {
            *v = Vec3::new(0.0, 0.0, 0.0);
        }
    }

    pub fn apply_light(&mut self, pos: Point3, color: Vec3) {
        let light_pos = Point3i::new(pos.x as i32, pos.y as i32, pos.z as i32);
        for (i, plane) in self.planes.planes_iter().enumerate() {
            let trace_pos = plane.cell + plane.dir.get_normal();

            let d = (pos - Point3::new(trace_pos.x as f32, trace_pos.y as f32, trace_pos.z as f32))
                .normalize();

            // normalize: make directional light
            let len = d.magnitude();
            // d /= len;
            let dot = nalgebra::Matrix::dot(&d, &plane.dir.get_normal());

            //self.emit[i] = Vec3::zero(); //new(0.2, 0.2, 0.2);
            let diff_color = self.diffuse[i];
            if !util::occluded(light_pos, trace_pos, &self.bitmap) && dot > 0f32 {
                // println!("light");
                self.emit[i] +=
                    vec_mul(&diff_color, &color) * dot * (5f32 / (2f32 * 3.1415f32 * len * len));
            }
        }
    }

    pub fn do_rad(&mut self) {
        self.do_rad_blocks();
        //self.do_rad_extents();
    }

    pub fn do_rad_extents(&mut self) {
        std::mem::swap(&mut self.rad_front, &mut self.rad_back);

        for (i, extents) in self.extents.iter().enumerate() {
            let mut rad_r = 0f32;
            let mut rad_g = 0f32;
            let mut rad_b = 0f32;
            let diffuse = self.diffuse[i as usize];

            let RadBuffer { r, g, b } = &self.rad_back;
            for ffs::Extent { start, ffs } in extents {
                for (j, ff) in ffs.iter().enumerate() {
                    rad_r += r[j + *start as usize] * diffuse.x * *ff;
                    rad_g += g[j + *start as usize] * diffuse.y * *ff;
                    rad_b += b[j + *start as usize] * diffuse.z * *ff;
                }
                self.pints += ffs.len();
            }

            self.rad_front.r[i as usize] = self.emit[i as usize].x + rad_r;
            self.rad_front.g[i as usize] = self.emit[i as usize].y + rad_g;
            self.rad_front.b[i as usize] = self.emit[i as usize].z + rad_b;
        }
    }

    pub fn do_rad_blocks(&mut self) {
        // let start = Instant::now();

        std::mem::swap(&mut self.rad_front, &mut self.rad_back);
        // self.rad_front.copy

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

        self.pints += tmp
            .par_iter_mut()
            // .iter_mut()
            .map(|(ref mut r, ref mut g, ref mut b, blocks, emit, diffuse)| {
                RadWorkblockSimd::new(self.rad_back.slice_full(), (r, g, b), blocks, emit, diffuse)
                    .do_iter()
            })
            .sum::<usize>();

        std::mem::swap(&mut self.rad_front, &mut front);
    }

    pub fn print_stat(&self) {
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

            let vdiffuse_r = f32x2::splat(diffuse.x);
            let vdiffuse_g = f32x2::splat(diffuse.y);
            let vdiffuse_b = f32x2::splat(diffuse.z);
            let mut vsum_r = f32x2::splat(0f32);
            let mut vsum_g = f32x2::splat(0f32);
            let mut vsum_b = f32x2::splat(0f32);

            for (j, ff) in ff_i.vec2.iter().zip(&ff_i.vec2_ff) {
                let j = *j as usize;
                let jrange = j..j + 2;
                let ff = *ff;
                unsafe {
                    let vr = f32x2::from_slice_aligned_unchecked(r.get_unchecked(jrange.clone()));
                    let vg = f32x2::from_slice_aligned_unchecked(g.get_unchecked(jrange.clone()));
                    let vb = f32x2::from_slice_aligned_unchecked(b.get_unchecked(jrange.clone()));

                    vsum_r += vdiffuse_r * ff * vr;
                    vsum_g += vdiffuse_g * ff * vg;
                    vsum_b += vdiffuse_b * ff * vb;
                }
            }

            rad_r += vsum_r.sum();
            rad_g += vsum_g.sum();
            rad_b += vsum_b.sum();

            let vdiffuse_r = f32x4::splat(diffuse.x);
            let vdiffuse_g = f32x4::splat(diffuse.y);
            let vdiffuse_b = f32x4::splat(diffuse.z);

            let mut vsum_r = f32x4::splat(0f32);
            let mut vsum_g = f32x4::splat(0f32);
            let mut vsum_b = f32x4::splat(0f32);

            for (j, ff) in ff_i.vec4.iter().zip(&ff_i.vec4_ff) {
                // unsafe {
                let j = *j as usize;
                let jrange = j..j + 4;
                let ff = *ff;
                unsafe {
                    let vr = f32x4::from_slice_aligned_unchecked(r.get_unchecked(jrange.clone()));
                    let vg = f32x4::from_slice_aligned_unchecked(g.get_unchecked(jrange.clone()));
                    let vb = f32x4::from_slice_aligned_unchecked(b.get_unchecked(jrange.clone()));

                    vsum_r += vdiffuse_r * ff * vr;
                    vsum_g += vdiffuse_g * ff * vg;
                    vsum_b += vdiffuse_b * ff * vb;
                }
            }
            rad_r += vsum_r.sum();
            rad_g += vsum_g.sum();
            rad_b += vsum_b.sum();

            let vdiffuse_r = f32x8::splat(diffuse.x);
            let vdiffuse_g = f32x8::splat(diffuse.y);
            let vdiffuse_b = f32x8::splat(diffuse.z);

            let mut vsum_r = f32x8::splat(0f32);
            let mut vsum_g = f32x8::splat(0f32);
            let mut vsum_b = f32x8::splat(0f32);

            for (j, ff) in ff_i.vec8.iter().zip(&ff_i.vec8_ff) {
                // unsafe {
                let j = *j as usize;
                let jrange = j..j + 8;
                let ff = *ff;
                unsafe {
                    let vr = f32x8::from_slice_aligned_unchecked(r.get_unchecked(jrange.clone()));
                    let vg = f32x8::from_slice_aligned_unchecked(g.get_unchecked(jrange.clone()));
                    let vb = f32x8::from_slice_aligned_unchecked(b.get_unchecked(jrange.clone()));

                    vsum_r += vdiffuse_r * ff * vr;
                    vsum_g += vdiffuse_g * ff * vg;
                    vsum_b += vdiffuse_b * ff * vb;
                }
            }
            rad_r += vsum_r.sum();
            rad_g += vsum_g.sum();
            rad_b += vsum_b.sum();

            let vdiffuse_r = f32x16::splat(diffuse.x);
            let vdiffuse_g = f32x16::splat(diffuse.y);
            let vdiffuse_b = f32x16::splat(diffuse.z);

            let mut vsum_r = f32x16::splat(0f32);
            let mut vsum_g = f32x16::splat(0f32);
            let mut vsum_b = f32x16::splat(0f32);

            for (j, ff) in ff_i.vec16.iter().zip(&ff_i.vec16_ff) {
                // unsafe {
                let j = *j as usize;
                let jrange = j..j + 16;
                let ff = *ff;
                unsafe {
                    let vr = f32x16::from_slice_aligned_unchecked(r.get_unchecked(jrange.clone()));
                    let vg = f32x16::from_slice_aligned_unchecked(g.get_unchecked(jrange.clone()));
                    let vb = f32x16::from_slice_aligned_unchecked(b.get_unchecked(jrange.clone()));

                    vsum_r += vdiffuse_r * ff * vr;
                    vsum_g += vdiffuse_g * ff * vg;
                    vsum_b += vdiffuse_b * ff * vb;
                }
            }
            rad_r += vsum_r.sum();
            rad_g += vsum_g.sum();
            rad_b += vsum_b.sum();

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
