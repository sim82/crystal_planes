use std::sync::{
    mpsc::Receiver, mpsc::SendError, mpsc::Sender, Arc, Mutex, RwLock, RwLockReadGuard,
    RwLockWriteGuard,
};

use super::{ffs, math::prelude::*, PlaneScene};
use bevy::prelude::*;

use super::rad_simd;
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
    let (rad_data, rad_to_render) = RadData::new(plane_scene, render_to_rad_channel);
    let front_buf = rad_data.front_buf.clone();
    rad_data.spawn();

    (front_buf, rad_to_render)
}

struct RadData {
    plane_scene: PlaneScene,
    render_to_rad_channel: Mutex<Receiver<RenderToRad>>,
    rad_to_render_channel: Sender<RadToRender>,
    num_planes: usize,
    back_buf: BackBuf,
    front_buf: FrontBuf,
    emit: Vec<Vec3>,
    diffuse: Vec<Vec3>,
    extents: Option<ffs::Extents>,
    formfactors: Option<Vec<(u32, u32, f32)>>,
    ff_recv: Option<Mutex<Receiver<Vec<(u32, u32, f32)>>>>,
    int_sum: usize,
    extents_simd: Option<Vec<rad_simd::ExtentsSimd>>,
    point_lights: std::collections::HashMap<usize, (Vec3, Vec3)>,
}

impl RadData {
    pub fn new(
        plane_scene: PlaneScene,
        render_to_rad_channel: Receiver<RenderToRad>,
    ) -> (RadData, Receiver<RadToRender>) {
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

        let (rad_to_render_channel, rand_to_render_recv) = std::sync::mpsc::channel();

        (
            RadData {
                plane_scene,
                render_to_rad_channel: Mutex::new(render_to_rad_channel),
                rad_to_render_channel: rad_to_render_channel,
                num_planes,
                back_buf,
                front_buf,
                emit,
                diffuse,
                extents: None,
                formfactors: None,
                ff_recv: None,
                int_sum: 0,
                extents_simd: None,
                point_lights: std::collections::HashMap::new(),
            },
            rand_to_render_recv,
        )
    }
}

trait RadThread {
    fn spawn(self);
}

trait RadGenerateExtents {
    fn start_generate(&mut self);
    fn update_generate(&mut self);
}

impl RadGenerateExtents for RadData {
    fn start_generate(&mut self) {
        match ffs::Extents::try_load("extents.bin", &self.plane_scene.get_digest()) {
            Some(extents) => {
                self.extents = Some(extents);
            }
            None => {
                self.ff_recv = Some(Mutex::new(ffs::generate_formfactors(
                    &self.plane_scene.planes,
                    self.plane_scene.blockmap.clone(),
                )));
            }
        };
    }

    fn update_generate(&mut self) {
        let mut drop_ff_recv = false;
        if let Some(recv) = self.ff_recv.as_ref() {
            let recv = recv.lock().unwrap();

            let now = std::time::Instant::now();
            while now.elapsed() < std::time::Duration::from_millis(10) {
                match recv.try_recv() {
                    Ok(mut ff) => {
                        // println!("ff: {:?}", ff);
                        let formfactors = self.formfactors.get_or_insert_with(|| Vec::new());
                        formfactors.append(&mut ff);
                        self.rad_to_render_channel
                            .send(RadToRender::StatusUpdate(format!(
                                "collecting fromfactors: {}",
                                formfactors.len()
                            )))
                            .unwrap();
                        // println!("append: {}", formfactors.len());
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => {
                        // println!("empty");
                        break;
                    }
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        let formfactors = self.formfactors.get_or_insert_with(|| Vec::new());
                        println!("done ffs: {}", formfactors.len());

                        self.rad_to_render_channel
                            .send(RadToRender::StatusUpdate("sort formfactors".into()))
                            .unwrap();
                        let formfactors = ffs::sort_formfactors(formfactors.clone());

                        self.rad_to_render_channel
                            .send(RadToRender::StatusUpdate("split formfactors".into()))
                            .unwrap();
                        let formfactors = ffs::split_formfactors(&formfactors);
                        self.rad_to_render_channel
                            .send(RadToRender::StatusUpdate("generate extents".into()))
                            .unwrap();
                        let extents = ffs::Extents(ffs::to_extents(&formfactors));
                        self.rad_to_render_channel
                            .send(RadToRender::StatusUpdate("write extents".into()))
                            .unwrap();
                        extents.write("extents.bin", &self.plane_scene.get_digest());
                        self.extents = Some(extents);

                        drop_ff_recv = true;
                        break;
                    }
                }
            }
        }
        if drop_ff_recv {
            self.ff_recv = None;
        }
    }
}

impl RadThread for RadData {
    fn spawn(mut self) {
        // let (rad_to_render_channel, rand_to_render_recv) = std::sync::mpsc::channel();
        // let mut last_emit_change = std::time::Instant::now();

        std::thread::spawn(move || {
            self.rad_to_render_channel
                .send(RadToRender::StatusUpdate("try load exents".into()))
                .unwrap();

            self.start_generate();

            let mut int_per_iter = 0;
            loop {
                self.update_generate();
                if self.extents.is_some() && int_per_iter == 0 {
                    int_per_iter = self
                        .extents
                        .as_ref()
                        .unwrap()
                        .0
                        .iter()
                        .flat_map(|es| es.iter().map(|e| e.ffs.len()))
                        .sum::<usize>();
                }

                if self.extents.is_some() && self.extents_simd.is_none() {
                    self.rad_to_render_channel
                        .send(RadToRender::StatusUpdate("generate simd extents".into()))
                        .unwrap();

                    let extens = self.extents.as_ref().unwrap();
                    let mut extents_simd = Vec::new();
                    for ext in &extens.0 {
                        let ext_simd = rad_simd::ExtentsSimd::from_extents(&ext);
                        extents_simd.push(ext_simd);
                    }
                    self.extents_simd = Some(extents_simd);
                    self.rad_to_render_channel
                        .send(RadToRender::RadReady)
                        .unwrap();
                }

                // only use last update of light 0 for now
                let mut light_updates = std::collections::HashSet::new();
                for cmd in self.render_to_rad_channel.lock().unwrap().try_iter() {
                    match cmd {
                        RenderToRad::PointLight(id, pos, color) => {
                            // ignore all but last light update
                            self.point_lights.insert(id, (pos, color));
                            light_updates.insert(id);
                        }
                        RenderToRad::SetStripeColors(color1, color2) => {
                            for (i, plane) in self.plane_scene.planes.planes_iter().enumerate() {
                                if ((plane.cell.y()) / 2) % 2 == 1 {
                                    continue;
                                }
                                self.diffuse[i] = match plane.dir {
                                    super::Dir::XyPos => color1,
                                    super::Dir::XyNeg => color2,
                                    super::Dir::YzPos | super::Dir::YzNeg => {
                                        Vec3::new(0.8f32, 0.8f32, 0.8f32)
                                    }
                                    _ => Vec3::new(1f32, 1f32, 1f32),
                                    // let color = hsv_to_rgb(rng.gen_range(0.0, 360.0), 1.0, 1.0); //random::<f32>(), 1.0, 1.0);
                                    // scene.diffuse[i] = Vector3::new(color.0, color.1, color.2);
                                }
                            }
                            // diffuse color change needs update of emit (via point lights)
                            light_updates.extend(self.point_lights.keys());
                        }
                    }
                }
                for id in light_updates.iter() {
                    if let Some((pos, color)) = self.point_lights.get(id) {
                        apply_pointlight(
                            &mut self.emit,
                            &self.diffuse,
                            &self.plane_scene,
                            pos,
                            color,
                        );
                    }
                }

                let rad_start = std::time::Instant::now();
                if self.extents_simd.is_some() {
                    self.rad_iter_extents_simd();
                    self.int_sum += int_per_iter;
                } else if self.extents.is_some() {
                    self.rad_iter_extents();
                    self.int_sum += int_per_iter;
                } else if self.formfactors.is_some() {
                    self.int_sum += self.rad_iter_raw_formfactors();
                }

                {
                    // swap back and front buffers. should be pretty much instant, so no blocking of gfx code.
                    let mut front = self.front_buf.write();
                    std::mem::swap(&mut *front, &mut self.back_buf.0);
                }
                match self.rad_to_render_channel.send(RadToRender::IterationDone {
                    num_int: self.int_sum,
                    duration: rad_start.elapsed(),
                }) {
                    Err(SendError(_)) => {
                        println!("channel disconnected. terminate rad thread");
                        break;
                    }
                    Ok(_) => (),
                };
                self.int_sum = 0;
            }
        });
    }
}
impl RadData {
    fn rad_iter_raw_formfactors(&mut self) -> usize {
        // println!("iter raw");
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
        // println!("len: {} {}", r.len(), front.r.len());
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
        formfactors.len()
    }

    fn rad_iter_extents(&mut self) {
        // println!("iter extents");
        let extents = self.extents.as_ref().unwrap();
        // run one iteration of radiosity integration (aka. 'heavy lifting').
        // holding only a read lock to front_buf, so gfx code can concurrently access it without blocking.
        let front = self.front_buf.read();

        let diffuse = &self.diffuse;
        let rad_out: Vec<(f32, f32, f32)> = (0..self.num_planes)
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
            self.back_buf.0.r[i] = self.emit[i].x() + rad_r;
            self.back_buf.0.g[i] = self.emit[i].y() + rad_g;
            self.back_buf.0.b[i] = self.emit[i].z() + rad_b;
        }
    }

    fn rad_iter_extents_simd(&mut self) {
        let extents_simd = self.extents_simd.as_ref().unwrap();
        // run one iteration of radiosity integration (aka. 'heavy lifting').
        // holding only a read lock to front_buf, so gfx code can concurrently access it without blocking.
        let front = self.front_buf.read();

        // let rad_out: Vec<(f32, f32, f32)> = (0..self.num_planes)
        //     .into_par_iter()
        //     .map(|i| {
        //         extents_simd[i].collect(
        //             i,
        //             (&front.r[..], &front.g[..], &front.b[..]),
        //             self.emit[i],
        //             self.diffuse[i],
        //         )
        //     })
        //     .collect();

        // for (i, (rad_r, rad_g, rad_b)) in rad_out.iter().enumerate() {
        //     self.back_buf.0.r[i] = *rad_r;
        //     self.back_buf.0.g[i] = *rad_g;
        //     self.back_buf.0.b[i] = *rad_b;
        // }

        let r = &mut self.back_buf.0.r;
        let g = &mut self.back_buf.0.g;
        let b = &mut self.back_buf.0.b;
        let emit = &self.emit;
        let diffuse = &self.diffuse;

        r.par_iter_mut()
            .zip(g.par_iter_mut())
            .zip(b.par_iter_mut())
            .enumerate()
            .for_each(|(i, ((r, g), b))| {
                let rad = extents_simd[i].collect(
                    i,
                    (&front.r[..], &front.g[..], &front.b[..]),
                    emit[i],
                    diffuse[i],
                );
                *r = rad.0;
                *g = rad.1;
                *b = rad.2;
            });
    }

    // fn send_status_update<T: Into<String>>(&mut self, text: T) {
    //     self.self.rad_to_render_channel
    //         .send(RadToRender::StatusUpdate(text)
    //         .unwrap();
    // }
}
