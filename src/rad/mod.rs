pub mod ffs;
pub mod simd;
pub mod worker;

// pub use ffs::*;
// pub use simd::*;
// pub use worker::*;

pub struct PlaneIndex {
    pub buf_index: usize,
}

pub mod data {
    use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
    pub struct RadBuffer {
        pub r: Vec<f32>,
        pub g: Vec<f32>,
        pub b: Vec<f32>,
    }

    impl RadBuffer {
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
}

pub mod com {
    use bevy::math::prelude::*;

    pub enum RenderToRad {
        PointLight(usize, Vec3, Vec3),
        SetStripeColors(Vec3, Vec3),
    }

    pub enum RadToRender {
        IterationDone {
            num_int: usize,
            duration: std::time::Duration,
        },
        StatusUpdate(String),
        RadReady,
    }
}

pub mod light {
    use bevy::math::prelude::*;

    use crate::{map::PlaneScene, math::prelude::*, util::vec_mul};
    pub fn apply_pointlight(
        emit: &mut Vec<Vec3>,
        diffuse: &Vec<Vec3>,
        plane_scene: &PlaneScene,
        pos: &Vec3,
        color: &Vec3,
    ) {
        let light_pos_i = Vec3i::from_vec3(&pos);
        for (i, plane) in plane_scene.planes.planes_iter().enumerate() {
            let trace_pos = plane.cell + plane.dir.get_normal_i(); // s

            let d = (*pos - trace_pos.into_vec3()).normalize();
            let len = 1f32;
            // normalize: make directional light
            // let len = d.length();
            // // d /= len;
            let dot = d.dot(plane.dir.get_normal());

            let diff_color = diffuse[i];
            // OPT-NOTE: tracing from plane to light has better change of early hit an allows for early termination if ray leaves map volume
            if !plane_scene
                .blockmap
                .occluded(trace_pos, light_pos_i, None, None, true)
                && dot > 0f32
            {
                // println!("light");
                emit[i] =
                    vec_mul(&diff_color, &color) * dot * (5f32 / (2f32 * 3.1415f32 * len * len));
            } else {
                emit[i] = Vec3::new(0f32, 0f32, 0f32);
            }
        }
    }
}
