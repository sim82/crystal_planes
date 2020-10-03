use super::ffs::Extent;
use packed_simd::{f32x4, f32x8};
struct ExtentsSimd {
    single: Vec<(u32, f32)>,
    vec4_ff: Vec<f32x4>,
    vec8_ff: Vec<f32x4>,
    vec4: Vec<u32>,
    vec8: Vec<u32>,
}

struct RadSlice<'a> {
    r: &'a [f32],
    g: &'a [f32],
    b: &'a [f32],
}

struct MutRadSlice<'a> {
    r: &'a mut [f32],
    g: &'a mut [f32],
    b: &'a mut [f32],
}

impl ExtentsSimd {
    pub fn from_extents(extents: &Vec<ffs::Extent>) -> ExtentsSimd {
        let mut vec8 = Vec::new();
        let mut vec4 = Vec::new();
        let mut vec8_ff = Vec::new();
        let mut vec4_ff = Vec::new();
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

        ExtentsSimd {
            single,
            vec4,
            vec8,
            vec4_ff,
            vec8_ff,
        }
    }

    pub fn collect(i: usize, src: RadSlice, dest: MutRadSlice, emit: &[Vec3], diffuse: &[Vec3]) {
        let mut rad_r = 0f32;
        let mut rad_g = 0f32;
        let mut rad_b = 0f32;
        let diffuse = self.diffuse[i];

        let (r, g, b) = self.src;
        for (j, ff) in &ff_i.single {
            unsafe {
                rad_r += r.get_unchecked(*j as usize) * diffuse.x * *ff;
                rad_g += g.get_unchecked(*j as usize) * diffuse.y * *ff;
                rad_b += b.get_unchecked(*j as usize) * diffuse.z * *ff;
            }
        }
        {
            type V = f32x4;

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
    }
}
