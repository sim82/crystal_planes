use super::ffs;
use bevy::math::prelude::*;
use packed_simd_2::{f32x16, f32x4, f32x8};

pub struct ExtentsCompressed {
    pub single: Vec<(u32, u8)>,
    vec4_ff: Vec<[u8; 4]>,
    vec8_ff: Vec<[u8; 8]>,
    vec16_ff: Vec<[u8; 16]>,
    pub vec4: Vec<u32>,
    pub vec8: Vec<u32>,
    pub vec16: Vec<u32>,
    buckets: [f32; 256],
}

type RadSlice<'a> = (&'a [f32], &'a [f32], &'a [f32]);

fn calc_histogram(extents: &[ffs::Extent]) -> [f32; 256] {
    fn log(v: f32) -> f32 {
        -(v.log2())
    }

    let ffs = extents
        .iter()
        .flat_map(|e| e.ffs.iter())
        .cloned()
        .collect::<Vec<f32>>();

    let min = ffs.iter().fold(f32::NAN, |a, v| a.min(*v));
    let max = ffs.iter().fold(f32::NAN, |a, v| a.max(*v));
    let n = 256;

    // let min = 0.1f32;
    // let max = 0.9f32;
    // let n = 2;
    // println!("minmax: {} {}", min, max);
    let minl = log(min);
    let maxl = log(max + 0.00001);
    let range_len = (maxl - minl) / (n as f32);

    let mut ret = [0f32; 256];

    ret.iter_mut()
        .enumerate()
        .for_each(|(i, v)| *v = (-(minl + (i + 1) as f32 * range_len)).exp2());
    ret
}
fn to_bucket(out: &mut [u8], v: &[f32], bins: &[f32; 256]) {
    for i in 0..out.len() {
        out[i] = bins
            .iter()
            .position(|bin| *bin >= v[i])
            .unwrap_or_else(|| panic!("cannot find bin for: {} in {:?}", v[i], bins))
            as u8;
    }
}

impl ExtentsCompressed {
    pub fn from_extents(extents: &Vec<ffs::Extent>) -> ExtentsCompressed {
        let mut vec16 = Vec::new();
        let mut vec8 = Vec::new();
        let mut vec4 = Vec::new();
        let mut vec16_ff = Vec::new();
        let mut vec8_ff = Vec::new();
        let mut vec4_ff = Vec::new();
        let mut single = Vec::new();

        let buckets = calc_histogram(extents);
        for ext in extents.iter().flat_map(|x| x.iter_aligned(&[16, 8, 4, 1])) {
            // for ext in extents.iter().flat_map(|x| x.iter_aligned(&[16, 1])) {
            match ext.ffs.len() {
                16 => {
                    vec16.push(ext.start);
                    let mut v = [0; 16];
                    to_bucket(&mut v, &ext.ffs, &buckets);
                    // println!("compress: {:?} -> {:?}", &ext.ffs, &v);
                    vec16_ff.push(v);
                }
                8 => {
                    vec8.push(ext.start);
                    let mut v = [0; 8];
                    to_bucket(&mut v, &ext.ffs, &buckets);
                    vec8_ff.push(v);
                }
                4 => {
                    vec4.push(ext.start);
                    let mut v = [0; 4];
                    to_bucket(&mut v, &ext.ffs, &buckets);
                    vec4_ff.push(v);
                }
                1 => {
                    let mut v = [0; 1];
                    to_bucket(&mut v, &ext.ffs, &buckets);
                    single.push((ext.start, v[0]))
                }
                _ => panic!("bad extent size: {}", ext.ffs.len()),
            }
        }

        ExtentsCompressed {
            single,
            vec4,
            vec8,
            vec16,
            vec4_ff,
            vec8_ff,
            vec16_ff,
            buckets,
        }
    }
    pub fn collect(
        &self,
        _i: usize,
        src: RadSlice,
        // dest: MutRadSlice,
        emit: Vec3,
        diffuse: Vec3,
    ) -> (f32, f32, f32) {
        let mut rad_r = 0f32;
        let mut rad_g = 0f32;
        let mut rad_b = 0f32;

        let (r, g, b) = src;
        for (j, ff) in &self.single {
            unsafe {
                rad_r += r.get_unchecked(*j as usize) * diffuse.x * self.buckets[*ff as usize];
                rad_g += g.get_unchecked(*j as usize) * diffuse.y * self.buckets[*ff as usize];
                rad_b += b.get_unchecked(*j as usize) * diffuse.z * self.buckets[*ff as usize];
            }
        }
        {
            let vdiffuse_r = f32x4::splat(diffuse.x);
            let vdiffuse_g = f32x4::splat(diffuse.y);
            let vdiffuse_b = f32x4::splat(diffuse.z);

            let mut vsum_r = f32x4::splat(0f32);
            let mut vsum_g = f32x4::splat(0f32);
            let mut vsum_b = f32x4::splat(0f32);

            for (j, ff) in self.vec4.iter().zip(&self.vec4_ff) {
                unsafe {
                    let ff = f32x4::new(
                        self.buckets[ff[0] as usize],
                        self.buckets[ff[1] as usize],
                        self.buckets[ff[2] as usize],
                        self.buckets[ff[3] as usize],
                    );
                    let j = *j as usize;
                    let vr = f32x4::from_slice_unaligned_unchecked(&r.get_unchecked(j..));
                    let vg = f32x4::from_slice_unaligned_unchecked(&g.get_unchecked(j..));
                    let vb = f32x4::from_slice_unaligned_unchecked(&b.get_unchecked(j..));

                    vsum_r += vdiffuse_r * ff * vr;
                    vsum_g += vdiffuse_g * ff * vg;
                    vsum_b += vdiffuse_b * ff * vb;
                }
            }
            rad_r += vsum_r.sum();
            rad_g += vsum_g.sum();
            rad_b += vsum_b.sum();
        }

        {
            let vdiffuse_r = f32x8::splat(diffuse.x);
            let vdiffuse_g = f32x8::splat(diffuse.y);
            let vdiffuse_b = f32x8::splat(diffuse.z);

            let mut vsum_r = f32x8::splat(0f32);
            let mut vsum_g = f32x8::splat(0f32);
            let mut vsum_b = f32x8::splat(0f32);

            for (j, ff) in self.vec8.iter().zip(&self.vec8_ff) {
                unsafe {
                    let ff = f32x8::new(
                        self.buckets[ff[0] as usize],
                        self.buckets[ff[1] as usize],
                        self.buckets[ff[2] as usize],
                        self.buckets[ff[3] as usize],
                        self.buckets[ff[4] as usize],
                        self.buckets[ff[5] as usize],
                        self.buckets[ff[6] as usize],
                        self.buckets[ff[7] as usize],
                    );
                    let j = *j as usize;
                    let vr = f32x8::from_slice_unaligned_unchecked(&r.get_unchecked(j..));
                    let vg = f32x8::from_slice_unaligned_unchecked(&g.get_unchecked(j..));
                    let vb = f32x8::from_slice_unaligned_unchecked(&b.get_unchecked(j..));

                    vsum_r += vdiffuse_r * ff * vr;
                    vsum_g += vdiffuse_g * ff * vg;
                    vsum_b += vdiffuse_b * ff * vb;
                }
            }
            rad_r += vsum_r.sum();
            rad_g += vsum_g.sum();
            rad_b += vsum_b.sum();
        }
        {
            let vdiffuse_r = f32x16::splat(diffuse.x);
            let vdiffuse_g = f32x16::splat(diffuse.y);
            let vdiffuse_b = f32x16::splat(diffuse.z);

            let mut vsum_r = f32x16::splat(0f32);
            let mut vsum_g = f32x16::splat(0f32);
            let mut vsum_b = f32x16::splat(0f32);

            for (j, ff) in self.vec16.iter().zip(&self.vec16_ff) {
                unsafe {
                    let ff = f32x16::new(
                        self.buckets[ff[0] as usize],
                        self.buckets[ff[1] as usize],
                        self.buckets[ff[2] as usize],
                        self.buckets[ff[3] as usize],
                        self.buckets[ff[4] as usize],
                        self.buckets[ff[5] as usize],
                        self.buckets[ff[6] as usize],
                        self.buckets[ff[7] as usize],
                        self.buckets[ff[8] as usize],
                        self.buckets[ff[9] as usize],
                        self.buckets[ff[10] as usize],
                        self.buckets[ff[11] as usize],
                        self.buckets[ff[12] as usize],
                        self.buckets[ff[13] as usize],
                        self.buckets[ff[14] as usize],
                        self.buckets[ff[15] as usize],
                    );
                    // let ff = f32x16::splat(0.0);
                    let j = *j as usize;
                    let vr = f32x16::from_slice_unaligned_unchecked(&r.get_unchecked(j..));
                    let vg = f32x16::from_slice_unaligned_unchecked(&g.get_unchecked(j..));
                    let vb = f32x16::from_slice_unaligned_unchecked(&b.get_unchecked(j..));

                    vsum_r += vdiffuse_r * ff * vr;
                    vsum_g += vdiffuse_g * ff * vg;
                    vsum_b += vdiffuse_b * ff * vb;
                }
            }
            rad_r += vsum_r.sum();
            rad_g += vsum_g.sum();
            rad_b += vsum_b.sum();
        }
        (rad_r + emit.x, rad_g + emit.y, rad_b + emit.z)
    }
}
