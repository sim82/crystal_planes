use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use super::{ffs, math::prelude::*, PlaneScene};
use bevy::prelude::*;
use rand::Rng;
use rayon::prelude::*;

pub struct RadBuffer {
    pub r: Vec<f32>,
    pub g: Vec<f32>,
    pub b: Vec<f32>,
}

impl RadBuffer {
    pub fn new(size: usize) -> RadBuffer {
        RadBuffer {
            r: vec![0.0; size],
            g: vec![0.0; size],
            b: vec![0.0; size],
        }
    }

    pub fn new_with(size: usize, r: f32, g: f32, b: f32) -> Self {
        RadBuffer {
            r: vec![r; size],
            g: vec![g; size],
            b: vec![b; size],
        }
    }
}

#[derive(Clone)]
pub struct FrontBuf(Arc<RwLock<RadBuffer>>);
pub struct BackBuf(pub RadBuffer);

impl FrontBuf {
    pub fn new(buf: RadBuffer) -> Self {
        FrontBuf(Arc::new(RwLock::new(buf)))
    }
    pub fn read(&self) -> RwLockReadGuard<RadBuffer> {
        self.0.read().unwrap()
    }
    pub fn write(&self) -> RwLockWriteGuard<RadBuffer> {
        self.0.write().unwrap()
    }
}

#[derive(Bundle)]
pub struct PlaneBundle {
    pub plane: Plane,
}
pub struct Plane {
    pub buf_index: usize,
}

pub fn hsv_to_rgb(h: f32, s: f32, v: f32) -> Vec3 {
    let h = if h == 360.0 { 0.0 } else { h / 60.0 };
    let fract = h - h.floor();

    let p = v * (1. - s);
    let q = v * (1. - s * fract);
    let t = v * (1. - s * (1. - fract));

    if h >= 0. && h < 1. {
        Vec3::new(v, t, p)
    } else if h >= 1. && h < 2. {
        Vec3::new(q, v, p)
    } else if h >= 2. && h < 3. {
        Vec3::new(p, v, t)
    } else if h >= 3. && h < 4. {
        Vec3::new(p, q, v)
    } else if h >= 4. && h < 5. {
        Vec3::new(t, p, v)
    } else if h >= 5. && h < 6. {
        Vec3::new(v, p, q)
    } else {
        Vec3::zero()
    }
}

pub enum RadUpdate {
    PointLight(usize, Vec3, Vec3),
}

pub fn apply_pointlight(
    emit: &mut Vec<Vec3>,
    diffuse: &Vec<Vec3>,
    plane_scene: &PlaneScene,
    pos: Vec3,
    color: Vec3,
) {
    let light_pos_i = Vec3i::from_vec3(&pos);
    for (i, plane) in plane_scene.planes.planes_iter().enumerate() {
        let trace_pos = plane.cell + plane.dir.get_normal_i(); // s

        let d = (pos - trace_pos.into_vec3()).normalize();
        let len = 1f32;
        // normalize: make directional light
        // let len = d.length();
        // // d /= len;
        let dot = d.dot(plane.dir.get_normal());

        let diff_color = diffuse[i];
        if !super::util::occluded(light_pos_i, trace_pos, &**plane_scene.blockmap) && dot > 0f32 {
            // println!("light");
            emit[i] = super::util::vec_mul(&diff_color, &color)
                * dot
                * (5f32 / (2f32 * 3.1415f32 * len * len));
        } else {
            emit[i] = Vec3::new(0f32, 0f32, 0f32);
        }
    }
}

pub fn spawn_rad_update(
    extents: ffs::Extents,
    plane_scene: PlaneScene,
    update_channel: std::sync::mpsc::Receiver<RadUpdate>,
) -> FrontBuf {
    let num_planes = extents.0.len();
    let front_buf = FrontBuf::new(RadBuffer::new_with(num_planes, 1.0, 0.5, 0.5));
    let mut back_buf = BackBuf(RadBuffer::new_with(num_planes, 0.5, 0.5, 1.0));

    let front_buf_ret = front_buf.clone();

    let mut diffuse = vec![Vec3::new(1.0, 1.0, 1.0); num_planes];

    let color1 = Vec3::new(1f32, 0.5f32, 0f32);
    let color2 = Vec3::new(0f32, 1f32, 0f32);
    for (i, plane) in plane_scene.planes.planes_iter().enumerate() {
        if ((plane.cell.y()) / 2) % 2 == 1 {
            continue;
        }
        diffuse[i] = match plane.dir {
            super::Dir::XyPos => color1,
            super::Dir::XyNeg => color2,
            super::Dir::YzPos | super::Dir::YzNeg => Vec3::new(0.8f32, 0.8f32, 0.8f32),
            _ => Vec3::new(1f32, 1f32, 1f32),
            // let color = hsv_to_rgb(rng.gen_range(0.0, 360.0), 1.0, 1.0); //random::<f32>(), 1.0, 1.0);
            // scene.diffuse[i] = Vector3::new(color.0, color.1, color.2);
        }
    }

    // let mut emit: Vec<Vec3> = (0..num_planes)
    //     .into_iter()
    //     .map(|_| {
    //         if rand::thread_rng().gen::<f32>() > 0.95 {
    //             Vec3::new(
    //                 rand::thread_rng().gen::<f32>(),
    //                 rand::thread_rng().gen::<f32>(),
    //                 rand::thread_rng().gen::<f32>(),
    //             )
    //         } else {
    //             Vec3::zero()
    //         }
    //     })
    //     .collect();

    let mut emit = vec![Vec3::zero(); num_planes];

    // let mut last_emit_change = std::time::Instant::now();
    std::thread::spawn(move || loop {
        // if last_emit_change.elapsed() > std::time::Duration::from_secs(1) {
        //     emit = (0..num_planes)
        //         .into_iter()
        //         .enumerate()
        //         .map(|(i, _)| {
        //             if rand::thread_rng().gen::<f32>() > 0.95 {
        //                 //hsv_to_rgb(rand::thread_rng().gen_range(0f32, 360f32), 1f32, 1f32)
        //                 diffuse[i]
        //             } else {
        //                 Vec3::zero()
        //             }
        //         })
        //         .collect();

        //     last_emit_change = std::time::Instant::now();
        // }

        // only use last update of light 0 for now
        match update_channel.try_iter().last() {
            Some(RadUpdate::PointLight(id, pos, color)) if id == 0 => {
                // println!("update: {} {:?}", id, pos);
                apply_pointlight(&mut emit, &diffuse, &plane_scene, pos, color);
            }
            _ => (),
        }

        {
            // run one iteration of radiosity integration (aka. 'heavy lifting').
            // holding only a read lock to front_buf, so gfx code can concurrently access it without blocking.
            let front = front_buf.read();

            let rad_out: Vec<(f32, f32, f32)> = (0..num_planes)
                .into_par_iter()
                .map(|i| {
                    let mut rad_r = 0f32;
                    let mut rad_g = 0f32;
                    let mut rad_b = 0f32;
                    let diffuse = diffuse[i];
                    for extent in &extents.0[i] {
                        for (j, ff) in extent.ffs.iter().enumerate() {
                            rad_r += front.r[j + extent.start as usize] * diffuse.x() * *ff;
                            rad_g += front.g[j + extent.start as usize] * diffuse.y() * *ff;
                            rad_b += front.b[j + extent.start as usize] * diffuse.z() * *ff;
                        }
                    }
                    (rad_r, rad_g, rad_b)
                })
                .collect();

            for (i, (rad_r, rad_g, rad_b)) in rad_out.iter().enumerate() {
                back_buf.0.r[i] = emit[i].x() + rad_r;
                back_buf.0.g[i] = emit[i].y() + rad_g;
                back_buf.0.b[i] = emit[i].z() + rad_b;
            }

            // emit.into_par_iter().enumerate().map(|(i, emit)| {
            //     emit + extents.0[i]
            //         .iter()
            //         .flat_map(|extent| {
            //             extent.ffs.iter().enumerate().map(move |(j, ff)| {
            //                 Vec3::new(
            //                     front.r[j + extent.start as usize] * diffuse[i].x() * *ff,
            //                     front.g[j + extent.start as usize] * diffuse[i].y() * *ff,
            //                     front.b[j + extent.start as usize] * diffuse[i].z() * *ff,
            //                 )
            //             })
            //         })
            //         .fold(Vec3::zero(), |a, v| a + v);
            // });
        }
        {
            // swap back and front buffers. should be pretty much instant, so no blocking of gfx code.
            let mut front = front_buf.write();
            std::mem::swap(&mut *front, &mut back_buf.0);
        }
    });
    front_buf_ret
}
