use crate::{
    crystal::{self, util, PlaneScene, Scene},
    math::prelude::*,
};
use amethyst::{
    core::{
        ecs::{
            Component, DenseVecStorage, Entities, Join, ReadExpect, ReadStorage, System,
            SystemData, WriteExpect,
        },
        math,
        //math::Point3,
        transform::Transform,
    },
    derive::SystemDesc,
    renderer::light::{Light, PointLight},
};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
type Index = u32;

pub struct MyPointLight {
    pos: Point3,
    color: Color,
}

impl Default for MyPointLight {
    fn default() -> MyPointLight {
        MyPointLight {
            pos: Point3::new(30.0, 30.0, 30.0),
            color: Color::new(1.0, 0.8, 0.8),
        }
    }
}

impl Component for MyPointLight {
    type Storage = DenseVecStorage<Self>;
}

#[derive(SystemDesc)]
#[system_desc(name(ApplyLightsSystemSystemDesc))]
pub struct ApplyLightsSystem;
impl<'a> System<'a> for ApplyLightsSystem {
    type SystemData = (
        WriteExpect<'a, Arc<Scene>>,
        ReadExpect<'a, Arc<PlaneScene>>,
        ReadStorage<'a, MyPointLight>,
    );

    fn run(&mut self, (rad_scene, plane_scene, point_lights): Self::SystemData) {
        let mut frontend = rad_scene.lock_frontend();
        frontend.clear_emit();
        for light in point_lights.join() {
            frontend.apply_light(
                &plane_scene.planes,
                &plane_scene.blockmap,
                &light.pos,
                &light.color,
            );
        }
    }
}

// #[derive(SystemDesc)]
// #[system_desc(name(ApplyRendyLightsSystemSystemDesc))]
// pub struct ApplyRendyLightsSystem;
// impl<'a> System<'a> for ApplyRendyLightsSystem {
//     type SystemData = (
//         Entities<'a>,
//         WriteExpect<'a, Arc<Scene>>,
//         ReadExpect<'a, Arc<PlaneScene>>,
//         ReadExpect<'a, Arc<LightWorker>>,
//         ReadStorage<'a, Light>,
//         ReadStorage<'a, Transform>,
//     );

//     fn run(
//         &mut self,
//         (ent, rad_scene, plane_scene, light_worker, light, transform): Self::SystemData,
//     ) {
//         let mut frontend = rad_scene.lock_frontend();
//         frontend.clear_emit();
//         for (ent, light, transform) in (&ent, &light, &transform).join() {
//             if let Light::Point(point_light) = light {
//                 println!("point light: {} {:?}", ent.id(), light);
//                 // FIXME: this is broken, and much too complicated for just getting the light's global translation...
//                 let pos = transform.global_view_matrix().try_inverse().unwrap()
//                     * Vec4::new(0.0, 0.0, 0.0, 1.0);
//                 // println!("transform: {:?}", transform);
//                 let pos = Point3::from_homogeneous(pos).unwrap();
//                 // println!("pos: {:?}", pos);
//                 frontend.apply_light(
//                     &plane_scene.planes,
//                     &plane_scene.blockmap,
//                     &pos,
//                     &Color::new(
//                         point_light.color.red,
//                         point_light.color.green,
//                         point_light.color.blue,
//                     ),
//                 );
//             }
//         }
//     }
// }

#[derive(SystemDesc)]
#[system_desc(name(ApplyRendyLightsSystemSystemDesc))]
pub struct ApplyRendyLightsSystem;
impl<'a> System<'a> for ApplyRendyLightsSystem {
    type SystemData = (
        Entities<'a>,
        WriteExpect<'a, Arc<Scene>>,
        ReadExpect<'a, Arc<PlaneScene>>,
        ReadExpect<'a, Arc<LightWorker>>,
        ReadStorage<'a, Light>,
        ReadStorage<'a, Transform>,
    );

    fn run(
        &mut self,
        (ent, rad_scene, plane_scene, light_worker, light, transform): Self::SystemData,
    ) {
        for (ent, light, transform) in (&ent, &light, &transform).join() {
            if let Light::Point(point_light) = light {
                println!("point light: {} {:?}", ent.id(), light);
                // FIXME: this is broken, and much too complicated for just getting the light's global translation...
                let pos = transform.global_view_matrix().try_inverse().unwrap()
                    * Vec4::new(0.0, 0.0, 0.0, 1.0);
                // println!("transform: {:?}", transform);
                let pos = Point3::from_homogeneous(pos).unwrap();
                // println!("pos: {:?}", pos);
                light_worker.update(ent.id(), (point_light.clone(), pos));
            }
        }
        let mut frontend = rad_scene.lock_frontend();
        frontend.emit = light_worker.get_output();
    }
}

pub struct LightWorker {
    light_requests: Mutex<VecDeque<(Index, PointLight, Point3)>>,
    num_planes: usize,
    emit: Mutex<HashMap<Index, Vec<Vec3>>>,
    output: Mutex<Vec<Vec3>>,
    plane_scene: Arc<PlaneScene>,
    diffuse: Vec<Vec3>,
}

impl LightWorker {
    pub fn new(plane_scene: Arc<PlaneScene>) -> Arc<Self> {
        let color1 = Vec3::new(1f32, 0.5f32, 0f32);
        // let color2 = hsv_to_rgb(rng.gen_range(0.0, 360.0), 1.0, 1.0);
        let color2 = Vec3::new(0f32, 1f32, 0f32);
        let diffuse = plane_scene
            .planes
            .planes_iter()
            .map(|plane| {
                if ((plane.cell.y) / 2) % 2 == 1 {
                    Vec3::new(1f32, 1f32, 1f32)
                } else {
                    match plane.dir {
                        crystal::Dir::XyPos => color1,
                        crystal::Dir::XyNeg => color2,
                        crystal::Dir::YzPos | crystal::Dir::YzNeg => {
                            Vec3::new(0.8f32, 0.8f32, 0.8f32)
                        }
                        _ => Vec3::new(1f32, 1f32, 1f32),
                    }
                }
            })
            .collect();
        let num_planes = plane_scene.planes.planes.len();
        let worker = Arc::new(LightWorker {
            light_requests: Mutex::new(VecDeque::new()),
            num_planes,
            emit: Mutex::new(HashMap::new()),
            output: Mutex::new(vec![Vec3::new(0f32, 0f32, 0f32); num_planes]),
            plane_scene,
            diffuse,
        });
        let wc = worker.clone();
        std::thread::spawn(move || {
            wc.run();
        });
        worker
    }

    pub fn update(&self, id: Index, update: (PointLight, Point3)) {
        if let Ok(mut req) = self.light_requests.lock() {
            let mut replaced = false;
            for (qid, ref mut point_light, ref mut pos) in req.iter_mut() {
                if *qid == id {
                    *point_light = update.0.clone();
                    *pos = update.1;
                    replaced = true;
                }
            }
            if !replaced {
                req.push_back((id, update.0, update.1));
            }
        }
    }

    fn run(&self) {
        loop {
            if let Ok(mut req) = self.light_requests.lock() {
                if let Some((id, point_light, pos)) = req.pop_front() {
                    let _pr = crystal::ProfTimer::new("light worker");

                    self.apply(id, point_light, pos);
                }
            }
        }
    }

    fn apply(&self, id: Index, point_light: PointLight, pos: Point3) {
        println!("apply: {} {:?} {:?} ", id, point_light, pos);
        if let Ok(mut emit) = self.emit.lock() {
            {
                let e = emit
                    .entry(id)
                    .or_insert_with(|| vec![Vec3::new(0.0, 0.0, 0.0); self.num_planes]);
                let light_pos = Point3i::new(pos.x as i32, pos.y as i32, pos.z as i32) * 4;
                for (i, plane) in self.plane_scene.planes.planes_iter().enumerate() {
                    let trace_pos = plane.cell + plane.dir.get_normal(); // s

                    let d = (pos
                        - Point3::new(trace_pos.x as f32, trace_pos.y as f32, trace_pos.z as f32))
                    .normalize();

                    // normalize: make directional light
                    let len = d.magnitude();
                    // d /= len;
                    let dot = math::Matrix::dot(&d, &plane.dir.get_normal());

                    let diff_color = self.diffuse[i];
                    if !util::occluded(light_pos, trace_pos, &self.plane_scene.blockmap)
                        && dot > 0f32
                    {
                        // println!("light");
                        e[i] = util::vec_mul(
                            &diff_color,
                            &Vec3::new(
                                point_light.color.red,
                                point_light.color.green,
                                point_light.color.blue,
                            ),
                        ) * dot
                            * (5f32 / (2f32 * 3.1415f32 * len * len));
                    } else {
                        e[i] = Vec3::new(0f32, 0f32, 0f32);
                    }
                }
            }
            // accumulate emit buffers to output
            if let Ok(mut output) = self.output.lock() {
                let emit_ref: Vec<&Vec<Vec3>> = emit.values().by_ref().collect();
                for (i, o) in output.iter_mut().enumerate() {
                    *o = emit_ref.iter().map(|e| e[i]).sum();
                }
            }
        }
    }
    pub fn get_output(&self) -> Vec<Vec3> {
        let output = self.output.lock().expect("light_worker output lock failed");
        output.clone()
    }
}
