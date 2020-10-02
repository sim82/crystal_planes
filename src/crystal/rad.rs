use std::sync::{mpsc::Receiver, Arc, Mutex, RwLock, RwLockReadGuard, RwLockWriteGuard};

use super::{ffs, math::prelude::*, PlaneScene};
use bevy::prelude::*;

use rayon::prelude::*;

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

#[derive(Bundle)]
pub struct PlaneBundle {
    pub plane: Plane,
}
pub struct Plane {
    pub buf_index: usize,
}

#[allow(dead_code)]
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

pub enum RenderToRad {
    PointLight(usize, Vec3, Vec3),
}

pub enum RadToRender {
    IterationDone {
        num_int: usize,
        duration: std::time::Duration,
    },
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
    plane_scene: PlaneScene,
    render_to_rad_channel: Receiver<RenderToRad>,
) -> (FrontBuf, Receiver<RadToRender>) {
    let rad_data = RadData::new(plane_scene, render_to_rad_channel);
    let front_buf = rad_data.front_buf.clone();
    let rad_to_render = rad_data.spawn();

    (front_buf, rad_to_render)
}

struct RadData {
    plane_scene: PlaneScene,
    render_to_rad_channel: Mutex<Receiver<RenderToRad>>,
    num_planes: usize,
    back_buf: BackBuf,
    front_buf: FrontBuf,
    emit: Vec<Vec3>,
    diffuse: Vec<Vec3>,
    extents: Option<ffs::Extents>,
    formfactors: Option<Vec<(u32, u32, f32)>>,
    ff_recv: Option<Mutex<Receiver<(u32, u32, f32)>>>,
    int_sum: usize,
}

impl RadData {
    pub fn new(plane_scene: PlaneScene, render_to_rad_channel: Receiver<RenderToRad>) -> RadData {
        let num_planes = plane_scene.planes.planes.len();
        let front_buf = FrontBuf::new(RadBuffer::new_with(num_planes, 1.0, 0.5, 0.5));
        let back_buf = BackBuf(RadBuffer::new_with(num_planes, 0.5, 0.5, 1.0));

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
        let emit = vec![Vec3::zero(); num_planes];

        RadData {
            plane_scene,
            render_to_rad_channel: Mutex::new(render_to_rad_channel),
            num_planes,
            back_buf,
            front_buf,
            emit,
            diffuse,
            extents: None,
            formfactors: None,
            ff_recv: None,
            int_sum: 0,
        }
    }
}

trait RadThread {
    fn spawn(self) -> Receiver<RadToRender>;
}

impl RadThread for RadData {
    fn spawn(mut self) -> Receiver<RadToRender> {
        let (rad_to_render_channel, rand_to_render_recv) = std::sync::mpsc::channel();
        // let mut last_emit_change = std::time::Instant::now();
        std::thread::spawn(move || {
            match ffs::Extents::load("extents.bin") {
                Some(extents) => self.extents = Some(extents),
                None => {
                    // let formfactors = ffs::setup_formfactors(
                    //     &self.plane_scene.planes,
                    //     &**self.plane_scene.blockmap,
                    // );
                    // self.formfactors = Some(ffs::sort_formfactors(formfactors));
                    // let formfactors = ffs::split_formfactors(self.formfactors.as_ref().unwrap());

                    self.ff_recv = Some(Mutex::new(ffs::generate_formfactors(
                        &self.plane_scene.planes,
                        self.plane_scene.blockmap.clone(),
                    )));
                    // let extents = ffs::Extents(ffs::to_extents(&formfactors));
                    // extents.write("extents.bin");
                }
            };
            // let extents = self.extents.as_ref().unwrap();

            // let int_per_iter = extents
            //     .0
            //     .iter()
            //     .flat_map(|es| es.iter().map(|e| e.ffs.len()))
            //     .sum::<usize>();
            let int_per_iter = 0;
            loop {
                if let Some(recv) = self.ff_recv.as_ref() {
                    let recv = recv.lock().unwrap();

                    let now = std::time::Instant::now();
                    while now.elapsed() < std::time::Duration::from_millis(10) {
                        match recv.try_recv() {
                            Ok(ff) => {
                                // println!("ff: {:?}", ff);
                                let formfactors =
                                    self.formfactors.get_or_insert_with(|| Vec::new());
                                formfactors.push(ff)
                            }
                            Err(std::sync::mpsc::TryRecvError::Empty) => {
                                // println!("empty");
                                break;
                            }
                            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                                // println!("done");
                                let formfactors =
                                    self.formfactors.get_or_insert_with(|| Vec::new());
                                let formfactors = ffs::sort_formfactors(formfactors.clone());
                                let formfactors = ffs::split_formfactors(&formfactors);
                                let extents = ffs::Extents(ffs::to_extents(&formfactors));
                                extents.write("extents.bin");
                                self.extents = Some(extents);
                                break;
                            }
                        }
                    }
                }

                // only use last update of light 0 for now
                match self.render_to_rad_channel.lock().unwrap().try_iter().last() {
                    Some(RenderToRad::PointLight(id, pos, color)) if id == 0 => {
                        // println!("update: {} {:?}", id, pos);
                        apply_pointlight(
                            &mut self.emit,
                            &self.diffuse,
                            &self.plane_scene,
                            pos,
                            color,
                        );
                    }
                    _ => (),
                }

                let rad_start = std::time::Instant::now();
                if self.extents.is_some() {
                    self.rad_iter_extents();
                    self.int_sum += int_per_iter;
                } else if self.formfactors.is_some() {
                    self.rad_iter_raw_formfactors();
                }

                {
                    // swap back and front buffers. should be pretty much instant, so no blocking of gfx code.
                    let mut front = self.front_buf.write();
                    std::mem::swap(&mut *front, &mut self.back_buf.0);
                }
                rad_to_render_channel
                    .send(RadToRender::IterationDone {
                        num_int: int_per_iter,
                        duration: rad_start.elapsed(),
                    })
                    .unwrap();
            }
        });
        rand_to_render_recv
    }
}
impl RadData {
    fn rad_iter_raw_formfactors(&mut self) {
        let r = &mut self.back_buf.0.r;
        let g = &mut self.back_buf.0.g;
        let b = &mut self.back_buf.0.b;
        for i in 0..r.len() {
            // FIXME: find out how to inplace set all Vec elements.
            r[i] = 0f32;
            g[i] = 0f32;
            b[i] = 0f32;
        }
        let formfactors = self.formfactors.as_ref().unwrap();

        let front = self.front_buf.read();
        println!("len: {} {}", r.len(), front.r.len());
        for (i, j, ff) in formfactors {
            let i = *i as usize;
            let j = *j as usize;
            let diffuse = self.diffuse[i];
            r[i] += front.r[j] * diffuse.x() * *ff;
            g[i] += front.g[j] * diffuse.y() * *ff;
            b[i] += front.b[j] * diffuse.z() * *ff;
        }
        for i in 0..r.len() {
            self.back_buf.0.r[i] += self.emit[i].x();
            self.back_buf.0.g[i] += self.emit[i].y();
            self.back_buf.0.b[i] += self.emit[i].z();
        }
    }

    fn rad_iter_extents(&mut self) {
        let extents = self.extents.as_ref().unwrap();
        // run one iteration of radiosity integration (aka. 'heavy lifting').
        // holding only a read lock to front_buf, so gfx code can concurrently access it without blocking.
        let front = self.front_buf.read();

        let rad_out: Vec<(f32, f32, f32)> = (0..self.num_planes)
            .into_par_iter()
            .map(|i| {
                let mut rad_r = 0f32;
                let mut rad_g = 0f32;
                let mut rad_b = 0f32;
                let diffuse = self.diffuse[i];
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
            self.back_buf.0.r[i] = self.emit[i].x() + rad_r;
            self.back_buf.0.g[i] = self.emit[i].y() + rad_g;
            self.back_buf.0.b[i] = self.emit[i].z() + rad_b;
        }
    }
}
