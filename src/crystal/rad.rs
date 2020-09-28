use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::crystal::ffs;
use bevy::{
    prelude::*,
    render::{
        mesh::{shape, VertexAttributeValues},
        pipeline::{DynamicBinding, PipelineDescriptor, PipelineSpecialization, RenderPipeline},
        render_graph::{base, AssetRenderResourcesNode, RenderGraph},
        renderer::RenderResources,
        shader::{ShaderStage, ShaderStages},
    },
};
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

pub fn spawn_rad_update(extents: ffs::Extents) -> FrontBuf {
    let num_planes = extents.0.len();
    let front_buf = FrontBuf::new(RadBuffer::new_with(num_planes, 1.0, 0.5, 0.5));
    let mut back_buf = BackBuf(RadBuffer::new_with(num_planes, 0.5, 0.5, 1.0));

    let front_buf_ret = front_buf.clone();

    let diffuse = vec![Vec3::new(1.0, 1.0, 1.0); num_planes];
    // let emit = vec![Vec3::new(1.0, 1.0, 1.0); num_planes];

    let mut emit: Vec<Vec3> = (0..num_planes)
        .into_iter()
        .map(|_| {
            if rand::thread_rng().gen::<f32>() > 0.95 {
                Vec3::new(
                    rand::thread_rng().gen::<f32>(),
                    rand::thread_rng().gen::<f32>(),
                    rand::thread_rng().gen::<f32>(),
                )
            } else {
                Vec3::zero()
            }
        })
        .collect();

    let mut last_emit_change = std::time::Instant::now();
    std::thread::spawn(move || loop {
        if last_emit_change.elapsed() > std::time::Duration::from_secs(4) {
            emit = (0..num_planes)
                .into_iter()
                .map(|_| {
                    if rand::thread_rng().gen::<f32>() > 0.95 {
                        Vec3::new(
                            rand::thread_rng().gen::<f32>(),
                            rand::thread_rng().gen::<f32>(),
                            rand::thread_rng().gen::<f32>(),
                        )
                    } else {
                        Vec3::zero()
                    }
                })
                .collect();

            last_emit_change = std::time::Instant::now();
        }

        {
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

            // std::thread::sleep_ms(300);
        }
        {
            let mut front = front_buf.write();
            std::mem::swap(&mut *front, &mut back_buf.0);
        }
    });
    front_buf_ret
}
