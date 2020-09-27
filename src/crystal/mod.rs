pub mod ffs;
// // pub mod misc;
// // pub mod rad;

// // mod rad_expv;
// // mod rad_impv;
// mod rad_noop;
// mod rad_par;
// mod rad_ref;
// mod rad_simdeez;
// use amethyst::core::math;
// // mod rad_stdsimd;
pub mod rads {
    // pub use super::rad_ref::*;
    // pub use super::rad_stdsimd::*;
    // pub use super::rad_impv::*;
    // pub use super::rad_par::*;
    // pub use super::rad_noop::*;
    // pub use super::rad_simdeez::*;
}
pub mod util;
// use crate::math::prelude::*;
// use std::iter::Iterator;

// mod buffer;
pub mod map;
pub mod math;
// mod scene;

// pub use buffer::{aligned_vector_init, MutRadSlice, RadBuffer, RadSlice};
pub use map::{read_map, Bitmap, BlockMap, Dir, Plane, PlaneScene, PlanesSep};
// pub use scene::{Scene, Stat};
// pub use util::ProfTimer;
// // simple profiling timer, e.g. to get an idea what is running when on which thread

// pub struct RadFrontend {
//     pub emit: Vec<Vec3>,
//     pub diffuse: Vec<Vec3>,
//     pub output: RadBuffer,
// }

// impl RadFrontend {
//     pub fn clear_emit(&mut self) {
//         for v in self.emit.iter_mut() {
//             *v = Vec3::new(0.0, 0.0, 0.0);
//         }
//     }

//     pub fn apply_light(
//         &mut self,
//         planes: &PlanesSep,
//         bitmap: &BlockMap,
//         pos: &Point3,
//         color: &Vec3,
//     ) {
//         // scale up light pos (each plane is only 0.25 * 0.25 in world space)
//         let light_pos = Point3i::new(pos.x as i32, pos.y as i32, pos.z as i32) * 4;
//         for (i, plane) in planes.planes_iter().enumerate() {
//             let trace_pos = plane.cell + plane.dir.get_normal(); // s

//             let d = (pos - Point3::new(trace_pos.x as f32, trace_pos.y as f32, trace_pos.z as f32))
//                 .normalize();

//             // normalize: make directional light
//             let len = d.magnitude();
//             // d /= len;
//             let dot = math::Matrix::dot(&d, &plane.dir.get_normal());

//             //self.emit[i] = Vec3::zero(); //new(0.2, 0.2, 0.2);
//             let diff_color = self.diffuse[i];
//             if !util::occluded(light_pos, trace_pos, &*bitmap) && dot > 0f32 {
//                 // println!("light");
//                 self.emit[i] += util::vec_mul(&diff_color, &color)
//                     * dot
//                     * (5f32 / (2f32 * 3.1415f32 * len * len));
//             }
//         }
//     }
// }

pub struct RadBuffer {
    r: Vec<f32>,
    g: Vec<f32>,
    b: Vec<f32>,
}
