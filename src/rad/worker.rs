use crate::{
    map::{self, PlaneScene},
    util::ProfTimer,
};

use bevy::prelude::*;
use std::sync::{mpsc::Receiver, mpsc::SendError, mpsc::Sender};

use super::{
    com::{RadToRender, RenderToRad},
    compress,
    data::{BackBuf, FrontBuf, RadBuffer},
    ffs,
    light::apply_pointlight,
    simd,
};
use rayon::prelude::*;
use tracing::info;

// the rad worker is built as a pipeline of different preprocessing steps. Each step
// implements the rad RadUpdate trait. The RadUpdate::update method can advance to the
// next step in the pipeline by returning a different RadUpdate object or it can
// re-schedule itself (note: (self: Box<Self>) signature -> teh update call consumes self).
//
// Current pipeline stages:
//
//          start
//            |
//        RadTryLoad
//            |
//     extents.bin exists?
//         |     |
//         |    no
//         |     |
//        yes    v
//         |   RadBuildFormfactorsParallel
//         |     |     |    ^
//         |    done   |    |
//         |     |      ----
//         v     v
//   RadGenerateSimdExtents
//            |
//            v
//      RadUpdateSimd
//         |     ^
//         |     |
//          -----

trait RadUpdate {
    fn update(self: Box<Self>) -> Option<Box<dyn RadUpdate>>;
    // fn finish(&mut self) -> Option<Box<dyn RadUpdate>>;
}

struct Channels {
    render_to_rad: Receiver<RenderToRad>,
    rad_to_render: Sender<RadToRender>,
}

impl Channels {
    fn send(&mut self, msg: RadToRender) -> bool {
        match self.rad_to_render.send(msg) {
            Err(SendError(_)) => false,
            Ok(_) => true,
        }
    }
}

struct CommonData {
    plane_scene: PlaneScene,
    back_buf: BackBuf,
    front_buf: FrontBuf,
    emit: Vec<Vec3>,
    diffuse: Vec<Vec3>,
    point_lights: std::collections::HashMap<usize, (Vec3, Vec3)>,
    lights_enabled: bool,
}

impl CommonData {
    fn apply_light_updates(&mut self, render_to_rad: &mut Receiver<RenderToRad>) {
        // only use last update of light 0 for now
        let mut light_updates = std::collections::HashSet::new();
        for cmd in render_to_rad.try_iter() {
            match cmd {
                RenderToRad::PointLight(id, pos, color) => {
                    // ignore all but last light update
                    self.point_lights.insert(id, (pos, color));
                    light_updates.insert(id);
                }
                RenderToRad::SetStripeColors(color1, color2) => {
                    for (i, plane) in self.plane_scene.planes.planes_iter().enumerate() {
                        if ((plane.cell.y) / 2) % 2 == 1 {
                            continue;
                        }
                        self.diffuse[i] = match plane.dir {
                            map::Dir::XyPos => color1,
                            map::Dir::XyNeg => color2,
                            map::Dir::YzPos | map::Dir::YzNeg => Vec3::new(0.8f32, 0.8f32, 0.8f32),
                            _ => Vec3::new(1f32, 1f32, 1f32),
                            // let color = hsv_to_rgb(rng.gen_range(0.0, 360.0), 1.0, 1.0); //random::<f32>(), 1.0, 1.0);
                            // scene.diffuse[i] = Vector3::new(color.0, color.1, color.2);
                        }
                    }
                    // diffuse color change needs update of emit (via point lights)
                    light_updates.extend(self.point_lights.keys());
                }
                RenderToRad::EnablePointlights(v) => {
                    self.lights_enabled = v;
                    light_updates.extend(self.point_lights.keys());
                    // println!("lights enabled: {}", self.lights_enabled)
                }
            }
        }
        if self.lights_enabled {
            for id in light_updates.iter() {
                if let Some((pos, color)) = self.point_lights.get(id) {
                    // let pt = crate::util::ProfTimer::new("apply_pointlight");
                    apply_pointlight(&mut self.emit, &self.diffuse, &self.plane_scene, pos, color);
                }
            }
        } else {
            self.emit.iter_mut().for_each(|v| *v = Vec3::ZERO)
        }
    }
}

struct RadTryLoad {
    channels: Channels,
    common: CommonData,
}

impl RadUpdate for RadTryLoad {
    fn update(mut self: Box<Self>) -> Option<Box<dyn RadUpdate>> {
        self.channels
            .send(RadToRender::StatusUpdate("try load extents.bin".into()));

        let _pt = ProfTimer::new("try read");
        match ffs::Extents::try_load("extents.bin", &self.common.plane_scene.get_digest()) {
            Some(extents) => Some(Box::new(RadGenerateSimdExtents {
                // Some(extents) => Some(Box::new(RadGenerateCompressedExtents {
                channels: self.channels,
                common: self.common,
                extents,
            })),
            None => Some(Box::new(RadBuildFormfactorsParallel::new(
                self.channels,
                self.common,
            ))),
        }
    }

    // fn finish(&mut self) -> Option<Box<dyn RadUpdate>> {
    //     todo!()
    // }
}

struct RadBuildFormfactors {
    channels: Channels,
    common: CommonData,
    iter: ffs::FormfactorBuildIterator,
    formfactors: Vec<(u32, u32, f32)>,
}
#[allow(dead_code)]
impl RadBuildFormfactors {
    fn new(channels: Channels, common: CommonData) -> Self {
        let iter = ffs::FormfactorBuildIterator::from_plane_scene(&common.plane_scene);
        RadBuildFormfactors {
            channels,
            common,
            iter,
            formfactors: Default::default(),
        }
    }
}
impl RadUpdate for RadBuildFormfactors {
    fn update(mut self: Box<Self>) -> Option<Box<dyn RadUpdate>> {
        let collect_start = std::time::Instant::now();
        loop {
            self.channels.send(RadToRender::StatusUpdate(format!(
                "build formfactors {}",
                self.formfactors.len()
            )));
            match self.iter.next() {
                Some(mut v) => {
                    self.formfactors.append(&mut v);
                }
                None => {
                    return Some(Box::new(RadPostprocessFormfactors {
                        channels: self.channels,
                        common: self.common,
                        formfactors: self.formfactors,
                    }));
                }
            }
            if collect_start.elapsed() > std::time::Duration::from_millis(1000) {
                break;
            }
        }
        self.common
            .apply_light_updates(&mut self.channels.render_to_rad);

        let rad_start = std::time::Instant::now();
        // println!("iter raw");
        let r_back = &mut self.common.back_buf.0.r;
        let g_back = &mut self.common.back_buf.0.g;
        let b_back = &mut self.common.back_buf.0.b;
        for i in 0..r_back.len() {
            // FIXME: find out how to inplace set all Vec elements.
            r_back[i] = 0f32;
            g_back[i] = 0f32;
            b_back[i] = 0f32;
        }
        let formfactors = &self.formfactors;
        {
            let front = self.common.front_buf.read();
            // println!("len: {} {}", r.len(), front.r.len());
            for (i, j, ff) in formfactors.iter() {
                let i = *i as usize;
                let j = *j as usize;
                let diffuse = self.common.diffuse[i];
                r_back[i] += front.r[j] * diffuse.x * *ff;
                g_back[i] += front.g[j] * diffuse.y * *ff;
                b_back[i] += front.b[j] * diffuse.z * *ff;
            }
            for i in 0..r_back.len() {
                self.common.back_buf.0.r[i] += self.common.emit[i].x;
                self.common.back_buf.0.g[i] += self.common.emit[i].y;
                self.common.back_buf.0.b[i] += self.common.emit[i].z;
            }
        }
        {
            // swap back and front buffers. should be pretty much instant, so no blocking of gfx code.
            let mut front = self.common.front_buf.write();
            std::mem::swap(&mut *front, &mut self.common.back_buf.0);
        }
        self.channels.send(RadToRender::IterationDone {
            duration: rad_start.elapsed(),
            num_int: formfactors.len(),
        });

        Some(self)
    }
}

struct RadBuildFormfactorsParallel {
    channels: Channels,
    common: CommonData,
}
impl RadBuildFormfactorsParallel {
    fn new(channels: Channels, common: CommonData) -> Self {
        RadBuildFormfactorsParallel { channels, common }
    }
}
impl RadUpdate for RadBuildFormfactorsParallel {
    fn update(mut self: Box<Self>) -> Option<Box<dyn RadUpdate>> {
        self.channels
            .send(RadToRender::StatusUpdate("build formfactors".into()));
        let formfactors = ffs::build_formfactors(
            &self.common.plane_scene.planes,
            self.common.plane_scene.blockmap.clone(),
        );
        rad_preview_on_plain_formfactors(&mut self.channels, &mut self.common, &formfactors);

        Some(Box::new(RadPostprocessFormfactors {
            channels: self.channels,
            common: self.common,
            formfactors,
        }))
    }
}

struct RadPostprocessFormfactors {
    channels: Channels,
    common: CommonData,
    formfactors: Vec<(u32, u32, f32)>,
}

impl RadUpdate for RadPostprocessFormfactors {
    fn update(mut self: Box<Self>) -> Option<Box<dyn RadUpdate>> {
        self.channels
            .send(RadToRender::StatusUpdate("sort formfactors".into()));

        let formfactors_sep = ffs::sort_formfactors(self.formfactors.clone());
        rad_preview_on_plain_formfactors(&mut self.channels, &mut self.common, &formfactors_sep);
        self.channels
            .send(RadToRender::StatusUpdate("split formfactors".into()));
        let formfactors = ffs::split_formfactors(&formfactors_sep);
        rad_preview_on_plain_formfactors(&mut self.channels, &mut self.common, &formfactors_sep);
        self.channels
            .send(RadToRender::StatusUpdate("build extents".into()));
        rad_preview_on_plain_formfactors(&mut self.channels, &mut self.common, &formfactors_sep);
        let extents = ffs::Extents(ffs::to_extents(&formfactors));
        self.channels
            .send(RadToRender::StatusUpdate("write extents.bin".into()));
        rad_preview_on_plain_formfactors(&mut self.channels, &mut self.common, &formfactors_sep);
        {
            let _pt = ProfTimer::new("write extents.bin");
            extents.write("extents.bin", &self.common.plane_scene.get_digest());
        }
        rad_preview_on_plain_formfactors(&mut self.channels, &mut self.common, &formfactors_sep);

        if true {
            info!("-> simd");
            Some(Box::new(RadGenerateSimdExtents {
                channels: self.channels,
                common: self.common,
                extents,
            }))
        } else {
            info!("-> compressed");
            Some(Box::new(RadGenerateCompressedExtents {
                channels: self.channels,
                common: self.common,
                extents,
            }))
        }
    }
}

struct RadGenerateSimdExtents {
    channels: Channels,
    common: CommonData,
    extents: ffs::Extents,
}

impl RadUpdate for RadGenerateSimdExtents {
    fn update(mut self: Box<Self>) -> Option<Box<dyn RadUpdate>> {
        self.channels
            .send(RadToRender::StatusUpdate("build simd extents".into()));
        let _pt = ProfTimer::new("to_simd_extents");
        let mut extents_simd = Vec::new();
        let mut num16 = 0;
        let mut num8 = 0;
        let mut num4 = 0;
        let mut num_single = 0;
        for ext in &self.extents.0 {
            let ext_simd = simd::ExtentsSimd::from_extents(&ext);
            num16 += ext_simd.vec16.len();
            num8 += ext_simd.vec8.len();
            num4 += ext_simd.vec4.len();
            num_single += ext_simd.single.len();
            extents_simd.push(ext_simd);
        }
        // extents_simd = self
        //     .extents
        //     .0
        //     .par_iter()
        //     .map(|ext| simd::ExtentsSimd::from_extents(&ext))
        //     .collect();
        info!(
            "extents:\n16 * {} = {}\n8 * {} = {}\n4 * {} = {}\n1 * {}",
            num16,
            num16 * 16,
            num8,
            num8 * 8,
            num4,
            num4 * 4,
            num_single
        );
        let int_per_iter = num16 * 16 + num8 * 8 + num4 * 4 + num_single;
        self.channels.send(RadToRender::RadReady);
        Some(Box::new(RadUpdateSimd {
            channels: self.channels,
            common: self.common,
            extents_simd,
            int_per_iter,
        }))
    }
}

struct RadGenerateCompressedExtents {
    channels: Channels,
    common: CommonData,
    extents: ffs::Extents,
}

impl RadUpdate for RadGenerateCompressedExtents {
    fn update(mut self: Box<Self>) -> Option<Box<dyn RadUpdate>> {
        self.channels
            .send(RadToRender::StatusUpdate("build compressed extents".into()));
        let _pt = ProfTimer::new("to_compressed_extents");
        let mut extents_simd = Vec::new();
        let mut num16 = 0;
        let mut num8 = 0;
        let mut num4 = 0;
        let mut num_single = 0;
        for ext in &self.extents.0 {
            let ext_simd = compress::ExtentsCompressed::from_extents(&ext);
            num16 += ext_simd.vec16.len();
            num8 += ext_simd.vec8.len();
            num4 += ext_simd.vec4.len();
            num_single += ext_simd.single.len();
            extents_simd.push(ext_simd);
        }

        // extents_simd = self
        //     .extents
        //     .0
        //     .par_iter()
        //     .map(|ext| compress::ExtentsCompressed::from_extents(&ext))
        //     .collect();
        info!(
            "extents:\n16 * {} = {}\n8 * {} = {}\n4 * {} = {}\n1 * {}",
            num16,
            num16 * 16,
            num8,
            num8 * 8,
            num4,
            num4 * 4,
            num_single
        );
        let int_per_iter = num16 * 16 + num8 * 8 + num4 * 4 + num_single;
        self.channels.send(RadToRender::RadReady);
        Some(Box::new(RadUpdateCompressed {
            channels: self.channels,
            common: self.common,
            extents_simd,
            int_per_iter,
        }))
    }
}

struct RadUpdateSimd {
    channels: Channels,
    common: CommonData,
    extents_simd: Vec<simd::ExtentsSimd>,
    int_per_iter: usize,
}

impl RadUpdate for RadUpdateSimd {
    fn update(mut self: Box<Self>) -> Option<Box<dyn RadUpdate>> {
        // panic!("end");

        self.common
            .apply_light_updates(&mut self.channels.render_to_rad);

        let rad_start = std::time::Instant::now();
        {
            let extents_simd = &mut self.extents_simd;
            // run one iteration of radiosity integration (aka. 'heavy lifting').
            // holding only a read lock to front_buf, so gfx code can concurrently access it without blocking.
            let front = self.common.front_buf.read();

            let r = &mut self.common.back_buf.0.r;
            let g = &mut self.common.back_buf.0.g;
            let b = &mut self.common.back_buf.0.b;
            let emit = &self.common.emit;
            let diffuse = &self.common.diffuse;

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
        {
            // swap back and front buffers. should be pretty much instant, so no blocking of gfx code.
            let mut front = self.common.front_buf.write();
            std::mem::swap(&mut *front, &mut self.common.back_buf.0);
        }
        if !self.channels.send(RadToRender::IterationDone {
            num_int: self.int_per_iter,
            duration: rad_start.elapsed(),
        }) {
            info!("channel disconnected. terminate rad thread");
            return None;
        }
        Some(self)
    }
}

struct RadUpdateCompressed {
    channels: Channels,
    common: CommonData,
    extents_simd: Vec<compress::ExtentsCompressed>,
    int_per_iter: usize,
}

impl RadUpdate for RadUpdateCompressed {
    fn update(mut self: Box<Self>) -> Option<Box<dyn RadUpdate>> {
        // panic!("end");

        self.common
            .apply_light_updates(&mut self.channels.render_to_rad);

        let rad_start = std::time::Instant::now();
        {
            let extents_simd = &mut self.extents_simd;
            // run one iteration of radiosity integration (aka. 'heavy lifting').
            // holding only a read lock to front_buf, so gfx code can concurrently access it without blocking.
            let front = self.common.front_buf.read();

            let r = &mut self.common.back_buf.0.r;
            let g = &mut self.common.back_buf.0.g;
            let b = &mut self.common.back_buf.0.b;
            let emit = &self.common.emit;
            let diffuse = &self.common.diffuse;

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
        {
            // swap back and front buffers. should be pretty much instant, so no blocking of gfx code.
            let mut front = self.common.front_buf.write();
            std::mem::swap(&mut *front, &mut self.common.back_buf.0);
        }
        if !self.channels.send(RadToRender::IterationDone {
            num_int: self.int_per_iter,
            duration: rad_start.elapsed(),
        }) {
            info!("channel disconnected. terminate rad thread");
            return None;
        }
        Some(self)
    }
}

pub fn spawn_rad_update(
    plane_scene: PlaneScene,
    render_to_rad_channel: Receiver<RenderToRad>,
) -> (FrontBuf, Receiver<RadToRender>) {
    let (rad_to_render_channel, rad_to_render_recv) = std::sync::mpsc::channel();
    let channels = Channels {
        render_to_rad: render_to_rad_channel,
        rad_to_render: rad_to_render_channel,
    };

    let num_planes = plane_scene.planes.planes.len();
    let front_buf = FrontBuf::new(RadBuffer::new_with(num_planes, 1.0, 0.5, 0.5));
    let back_buf = BackBuf(RadBuffer::new_with(num_planes, 0.5, 0.5, 1.0));

    let mut diffuse = vec![Vec3::new(1.0, 1.0, 1.0); num_planes];

    let color1 = Vec3::new(1f32, 0.5f32, 0f32);
    let color2 = Vec3::new(0f32, 1f32, 0f32);
    for (i, plane) in plane_scene.planes.planes_iter().enumerate() {
        if ((plane.cell.y) / 2) % 2 == 1 {
            continue;
        }
        diffuse[i] = match plane.dir {
            map::Dir::XyPos => color1,
            map::Dir::XyNeg => color2,
            map::Dir::YzPos | map::Dir::YzNeg => Vec3::new(0.8f32, 0.8f32, 0.8f32),
            _ => Vec3::new(1f32, 1f32, 1f32),
            // let color = hsv_to_rgb(rng.gen_range(0.0, 360.0), 1.0, 1.0); //random::<f32>(), 1.0, 1.0);
            // scene.diffuse[i] = Vector3::new(color.0, color.1, color.2);
        }
    }
    let emit = vec![Vec3::ZERO; num_planes];
    let front_buf_clone = front_buf.clone();
    let common = CommonData {
        plane_scene,
        back_buf,
        front_buf,
        emit,
        diffuse,
        point_lights: std::collections::HashMap::new(),
        lights_enabled: true,
    };

    std::thread::spawn(move || {
        let mut update = Box::new(RadTryLoad { channels, common }) as Box<dyn RadUpdate>;
        while let Some(new_update) = update.update() {
            update = new_update
        }
    });

    // let (rad_data, rad_to_render) = RadData::new(plane_scene, render_to_rad_channel);
    // let front_buf = rad_data.front_buf.clone();
    // rad_data.spawn();

    (front_buf_clone, rad_to_render_recv)
}

fn rad_preview_on_plain_formfactors(
    channels: &mut Channels,
    common: &mut CommonData,
    formfactors: &[(u32, u32, f32)],
) {
    let start = std::time::Instant::now();
    loop {
        common.apply_light_updates(&mut channels.render_to_rad);
        let rad_start = std::time::Instant::now();
        // println!("iter raw");
        let r_back = &mut common.back_buf.0.r;
        let g_back = &mut common.back_buf.0.g;
        let b_back = &mut common.back_buf.0.b;
        for i in 0..r_back.len() {
            // FIXME: find out how to inplace set all Vec elements.
            r_back[i] = 0f32;
            g_back[i] = 0f32;
            b_back[i] = 0f32;
        }
        // let formfactors = &self.formfactors;
        {
            let front = common.front_buf.read();
            // println!("len: {} {}", r.len(), front.r.len());
            for (i, j, ff) in formfactors.iter() {
                let i = *i as usize;
                let j = *j as usize;
                let diffuse = common.diffuse[i];
                r_back[i] += front.r[j] * diffuse.x * *ff;
                g_back[i] += front.g[j] * diffuse.y * *ff;
                b_back[i] += front.b[j] * diffuse.z * *ff;
            }
            for i in 0..r_back.len() {
                common.back_buf.0.r[i] += common.emit[i].x;
                common.back_buf.0.g[i] += common.emit[i].y;
                common.back_buf.0.b[i] += common.emit[i].z;
            }
        }
        {
            // swap back and front buffers. should be pretty much instant, so no blocking of gfx code.
            let mut front = common.front_buf.write();
            std::mem::swap(&mut *front, &mut common.back_buf.0);
        }
        channels.send(RadToRender::IterationDone {
            duration: rad_start.elapsed(),
            num_int: formfactors.len(),
        });
        if start.elapsed().as_millis() > 20 {
            break;
        }
    }
}
