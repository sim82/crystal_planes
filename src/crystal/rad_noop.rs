use super::ffs::{self, Extent};
#[allow(unused_imports)]
use super::{
    aligned_vector_init, Bitmap, BlockMap, DisplayWrap, MutRadSlice, RadBuffer, RadFrontend,
    RadSlice,
};
use crate::math::prelude::*;

use std::sync::Mutex;

#[allow(unused)]
pub struct RadBackend {
    pub emit: Vec<Vec3>,
    pub extents: Vec<Vec<ffs::Extent>>,
    pub rad_front: RadBuffer,
    pub rad_back: RadBuffer,
    pub diffuse: Vec<Vec3>,
}

#[allow(unused)]
impl RadBackend {
    pub fn new(extents: Vec<Vec<Extent>>) -> Self {
        let num_planes = extents.len();
        RadBackend {
            emit: vec![Vec3::new(0.0, 0.0, 0.0); num_planes],
            rad_front: RadBuffer::new(num_planes),
            rad_back: RadBuffer::new(num_planes),
            extents: extents,
            diffuse: vec![Vec3::new(1f32, 1f32, 1f32); num_planes],
        }
    }

    pub fn do_rad(&mut self, frontend: &Mutex<RadFrontend>) -> usize {
        self.do_rad_extents(frontend)
    }

    pub fn do_rad_extents(&mut self, frontend: &Mutex<RadFrontend>) -> usize {
        {
            let mut frontend = frontend.lock().expect("rad frontend lock failed");

            frontend.output = self.rad_back.clone();
            self.emit = frontend.emit.clone();
            self.diffuse = frontend.diffuse.clone();
        }
        std::mem::swap(&mut self.rad_front, &mut self.rad_back);

        let mut pint = 0;
        for (i, extents) in self.extents.iter().enumerate() {
            self.rad_front.r[i as usize] = self.emit[i as usize].x;
            self.rad_front.g[i as usize] = self.emit[i as usize].y;
            self.rad_front.b[i as usize] = self.emit[i as usize].z;
        }
        pint
    }
}
