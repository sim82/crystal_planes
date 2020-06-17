use amethyst::core::ecs::{ReadExpect, SystemData, World};

use super::{ffs, rads, PlaneScene, RadBuffer, RadFrontend, Vec3};
use std::sync::{Arc, Mutex};

pub struct Stat {
    pints: usize,
    last_stat: Option<std::time::Instant>,
}

pub struct Scene {
    pub internal: Mutex<super::rads::RadBackend>,
    pub frontend: Mutex<super::RadFrontend>,
    pub stat: Mutex<Stat>,
}

impl Scene {
    pub fn new(world: &World) -> Self {
        let plane_scene = <(ReadExpect<Arc<PlaneScene>>)>::fetch(world);

        let filename = "extents.bin";

        let extents = if let Some(extents) = ffs::load_extents(filename) {
            extents
        } else {
            let formfactors = ffs::split_formfactors(ffs::setup_formfactors(
                &plane_scene.planes,
                &plane_scene.blockmap,
            ));
            let extents = ffs::to_extents(&formfactors);
            ffs::write_extents(filename, &extents);
            println!("wrote {}", filename);
            extents
        };

        let internal = rads::RadBackend::new(extents);

        Scene {
            internal: Mutex::new(internal),
            frontend: Mutex::new(RadFrontend {
                emit: vec![Vec3::new(0.0, 0.0, 0.0); plane_scene.planes.num_planes()],
                diffuse: vec![Vec3::new(1f32, 1f32, 1f32); plane_scene.planes.num_planes()],
                output: RadBuffer::new(plane_scene.planes.num_planes()),
            }),
            stat: Mutex::new(Stat {
                pints: 0,
                last_stat: None,
            }),
        }
    }
    pub fn lock_frontend(&self) -> std::sync::MutexGuard<'_, RadFrontend> {
        self.frontend.lock().expect("rad frontend lock failed")
    }
    pub fn do_rad(&self) {
        let pint = {
            let mut internal = self.internal.lock().expect("lock internal failed");
            internal.do_rad(&self.frontend)
        };

        if let Ok(ref mut stat) = self.stat.lock() {
            stat.pints += pint;
            let mut new_time = false;
            if let Some(time) = stat.last_stat {
                let elapsed = time.elapsed();
                if elapsed >= std::time::Duration::from_secs(1) {
                    let ps = stat.pints as f64 / elapsed.as_secs_f64();
                    println!("pint/s: {}", ps);
                    new_time = true;
                    stat.pints = 0;
                }
            } else {
                new_time = true;
            }

            if new_time {
                stat.last_stat = Some(std::time::Instant::now());
            }
        }
    }
}
