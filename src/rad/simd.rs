use super::ffs;
use bevy::math::prelude::*;
use packed_simd_2::{f32x16, f32x4, f32x8};
pub struct ExtentsSimd {
    pub single: Vec<(u32, f32)>,
    vec4_ff: Vec<f32x4>,
    vec8_ff: Vec<f32x8>,
    vec16_ff: Vec<f32x16>,
    pub vec4: Vec<u32>,
    pub vec8: Vec<u32>,
    pub vec16: Vec<u32>,
}

// struct RadSlice<'a>(&'a [f32], &'a [f32], &'a [f32]);
type RadSlice<'a> = (&'a [f32], &'a [f32], &'a [f32]);

impl ExtentsSimd {
    pub fn from_extents(extents: &Vec<ffs::Extent>) -> ExtentsSimd {
        let mut vec16 = Vec::new();
        let mut vec8 = Vec::new();
        let mut vec4 = Vec::new();
        let mut vec16_ff = Vec::new();
        let mut vec8_ff = Vec::new();
        let mut vec4_ff = Vec::new();
        let mut single = Vec::new();

        for ext in extents.iter().flat_map(|x| x.split_aligned(&[16, 8, 4, 1])) {
            match ext.ffs.len() {
                16 => {
                    vec16.push(ext.start);
                    vec16_ff.push(f32x16::from_slice_unaligned(&ext.ffs));
                }
                8 => {
                    vec8.push(ext.start);
                    vec8_ff.push(f32x8::from_slice_unaligned(&ext.ffs));
                }
                4 => {
                    vec4.push(ext.start);
                    vec4_ff.push(f32x4::from_slice_unaligned(&ext.ffs));
                }
                1 => single.push((ext.start, ext.ffs[0])),
                _ => panic!("bad extent size: {}", ext.ffs.len()),
            }
        }

        ExtentsSimd {
            single,
            vec4,
            vec8,
            vec16,
            vec4_ff,
            vec8_ff,
            vec16_ff,
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
                rad_r += r.get_unchecked(*j as usize) * diffuse.x() * *ff;
                rad_g += g.get_unchecked(*j as usize) * diffuse.y() * *ff;
                rad_b += b.get_unchecked(*j as usize) * diffuse.z() * *ff;
            }
        }
        {
            let vdiffuse_r = f32x4::splat(diffuse.x());
            let vdiffuse_g = f32x4::splat(diffuse.y());
            let vdiffuse_b = f32x4::splat(diffuse.z());

            let mut vsum_r = f32x4::splat(0f32);
            let mut vsum_g = f32x4::splat(0f32);
            let mut vsum_b = f32x4::splat(0f32);

            for (j, ff) in self.vec4.iter().zip(&self.vec4_ff) {
                unsafe {
                    let j = *j as usize;
                    let vr = f32x4::from_slice_unaligned_unchecked(&r.get_unchecked(j..));
                    let vg = f32x4::from_slice_unaligned_unchecked(&g.get_unchecked(j..));
                    let vb = f32x4::from_slice_unaligned_unchecked(&b.get_unchecked(j..));

                    vsum_r += vdiffuse_r * *ff * vr;
                    vsum_g += vdiffuse_g * *ff * vg;
                    vsum_b += vdiffuse_b * *ff * vb;
                }
            }
            rad_r += vsum_r.sum();
            rad_g += vsum_g.sum();
            rad_b += vsum_b.sum();
        }

        {
            let vdiffuse_r = f32x8::splat(diffuse.x());
            let vdiffuse_g = f32x8::splat(diffuse.y());
            let vdiffuse_b = f32x8::splat(diffuse.z());

            let mut vsum_r = f32x8::splat(0f32);
            let mut vsum_g = f32x8::splat(0f32);
            let mut vsum_b = f32x8::splat(0f32);

            for (j, ff) in self.vec8.iter().zip(&self.vec8_ff) {
                unsafe {
                    let j = *j as usize;
                    let vr = f32x8::from_slice_unaligned_unchecked(&r.get_unchecked(j..));
                    let vg = f32x8::from_slice_unaligned_unchecked(&g.get_unchecked(j..));
                    let vb = f32x8::from_slice_unaligned_unchecked(&b.get_unchecked(j..));

                    vsum_r += vdiffuse_r * *ff * vr;
                    vsum_g += vdiffuse_g * *ff * vg;
                    vsum_b += vdiffuse_b * *ff * vb;
                }
            }
            rad_r += vsum_r.sum();
            rad_g += vsum_g.sum();
            rad_b += vsum_b.sum();
        }
        {
            let vdiffuse_r = f32x16::splat(diffuse.x());
            let vdiffuse_g = f32x16::splat(diffuse.y());
            let vdiffuse_b = f32x16::splat(diffuse.z());

            let mut vsum_r = f32x16::splat(0f32);
            let mut vsum_g = f32x16::splat(0f32);
            let mut vsum_b = f32x16::splat(0f32);

            for (j, ff) in self.vec16.iter().zip(&self.vec16_ff) {
                unsafe {
                    let j = *j as usize;
                    let vr = f32x16::from_slice_unaligned_unchecked(&r.get_unchecked(j..));
                    let vg = f32x16::from_slice_unaligned_unchecked(&g.get_unchecked(j..));
                    let vb = f32x16::from_slice_unaligned_unchecked(&b.get_unchecked(j..));

                    vsum_r += vdiffuse_r * *ff * vr;
                    vsum_g += vdiffuse_g * *ff * vg;
                    vsum_b += vdiffuse_b * *ff * vb;
                }
            }
            rad_r += vsum_r.sum();
            rad_g += vsum_g.sum();
            rad_b += vsum_b.sum();
        }
        // dest.0[i] = rad_r + emit[i].x();
        // dest.1[i] = rad_g + emit[i].y();
        // dest.2[i] = rad_b + emit[i].z();
        (rad_r + emit.x(), rad_g + emit.y(), rad_b + emit.z())
    }
}
