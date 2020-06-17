use crate::vertex::QuadInstanceArgsConst;
use amethyst::derive::SystemDesc;
#[allow(unused_imports)]
use amethyst::prelude::*;
use amethyst::{
    core::{
        ecs::{Component, DenseVecStorage, Join, System, SystemData, Write, WriteStorage},
        math::{Vector3, Vector4},
    },
    renderer::rendy::mesh::Color,
};
use rand::Rng; //prelude::*;

#[derive(Clone)]
pub struct QuadInstance {
    pub translate: Vector3<f32>,
    pub dir: u32,
    pub color: Vector4<f32>,
    pub index: u32, // temporary for plane sorting
}

impl QuadInstance {
    pub fn get_args(&self) -> Color {
        let color: [f32; 4] = self.color.into();
        // QuadInstanceArgs {
        //     color: color.into(),
        // }
        color.into()
    }
    pub fn get_args_const(&self) -> QuadInstanceArgsConst {
        let translate: [f32; 3] = self.translate.into();
        // let color: [f32; 4] = self.color.into();
        QuadInstanceArgsConst {
            translate: translate.into(),
            dir: self.dir.into(),
            // color: color.into(),
        }
    }
}

impl Component for QuadInstance {
    type Storage = DenseVecStorage<Self>;
}

pub struct ColorGeneration(pub usize);

#[derive(SystemDesc)]
#[system_desc(name(DiscoSystemDesc))]
pub struct DiscoSystem;
impl<'a> System<'a> for DiscoSystem {
    type SystemData = (
        WriteStorage<'a, QuadInstance>,
        Write<'a, Option<ColorGeneration>>,
    );

    fn run(&mut self, (mut quad_instances, mut color_generation): Self::SystemData) {
        let mut rand = rand::thread_rng();
        use random_color::{Luminosity, RandomColor};
        let mut rc = RandomColor::new();
        rc.luminosity(Luminosity::Bright);

        for q in (&mut quad_instances).join() {
            let color = if rand.gen_bool(0.1) {
                rc.to_rgb_array()
            } else {
                [0; 3]
            };
            q.color[0] = color[0] as f32 / 255.0;
            q.color[1] = color[1] as f32 / 255.0;
            q.color[2] = color[2] as f32 / 255.0;
        }
        if let Some(ref mut color_generation) = *color_generation {
            color_generation.0 += 1;
        }
    }
}
