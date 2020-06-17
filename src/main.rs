//! Demonstrates how to use the fly camera
#[macro_use]
extern crate itertools;

mod crystal;
mod custom_pass;
mod light;
mod math;
mod quad;
mod quad_pass;
mod systems;
mod vertex;
use crate::quad_pass::RenderQuad;
use amethyst::{
    animation::{
        get_animation_set, AnimationBundle, AnimationCommand, AnimationSet, AnimationSetPrefab,
        DeferStartRelation, EndControl,
    },
    assets::{PrefabLoader, PrefabLoaderSystemDesc, RonFormat},
    controls::{FlyControlBundle, HideCursor},
    core::{
        math::{Vector3, Vector4},
        transform::TransformBundle,
        Transform,
    },
    ecs::{prelude::*, WorldExt, WriteExpect},
    input::{is_key_down, is_mouse_button_down, InputBundle, StringBindings},
    prelude::*,
    renderer::{
        light::Light,
        palette::Srgb,
        plugins::{RenderShaded3D, RenderSkybox, RenderToWindow},
        rendy::mesh::{Normal, Position, TexCoord},
        types::DefaultBackend,
        RenderingBundle,
    },
    utils::{application_root_dir, auto_fov::AutoFovSystem, scene::BasicScenePrefab},
    winit::{MouseButton, VirtualKeyCode},
    Error,
};
use clap::{App, Arg};
use serde::{Deserialize, Serialize};
type MyPrefabData = (
    Option<BasicScenePrefab<(Vec<Position>, Vec<Normal>, Vec<TexCoord>)>>,
    Option<AnimationSetPrefab<AnimationId, Transform>>,
    Option<AnimationSetPrefab<AnimationId, Light>>,
);

#[derive(Eq, PartialOrd, PartialEq, Hash, Debug, Copy, Clone, Deserialize, Serialize)]
enum AnimationId {
    Scale,
    Rotate,
    Translate,
    Test,
}

#[derive(PartialEq, Debug)]
enum LightMode {
    RandomFlashing,
    Tron,
    LightSources,
    RendyLightSources,
}

impl Default for LightMode {
    fn default() -> Self {
        LightMode::RandomFlashing
    }
}

struct MapLoadState;
impl SimpleState for MapLoadState {
    fn update(&mut self, data: &mut StateData<'_, GameData<'_, '_>>) -> SimpleTrans {
        let world = &mut data.world;

        let bm = crystal::read_map("hidden_ramp.txt").expect("could not read file");
        let mut planes = crystal::PlanesSep::new();
        planes.create_planes(&bm);
        // let planes_copy: Vec<crystal::Plane> = planes.planes_iter().cloned().collect();
        world.register::<crystal::Plane>();
        world.register::<quad::QuadInstance>();
        world.register::<light::MyPointLight>();
        world.insert(Some(quad::ColorGeneration(0)));
        world.insert(LightMode::RandomFlashing);

        world
            .create_entity()
            .named("the MyPointLight")
            .with(light::MyPointLight::default())
            .build();

        for (i, p) in planes.planes_iter().cloned().enumerate() {
            let point = &p.cell;
            let dir = match p.dir {
                crystal::Dir::ZxPos => 4,
                crystal::Dir::ZxNeg => 5,
                crystal::Dir::YzPos => 2,
                crystal::Dir::YzNeg => 3,
                crystal::Dir::XyPos => 0,
                crystal::Dir::XyNeg => 1,
            };
            let quad = quad::QuadInstance {
                translate: Vector3::new(
                    point[0] as f32 * 0.25,
                    point[1] as f32 * 0.25,
                    point[2] as f32 * 0.25,
                ),
                dir,
                color: Vector4::new(1.0, 1.0, 1.0, 1.0),
                index: i as u32,
            };
            world.create_entity().with(p).with(quad).build();
        }
        let plane_scene = std::sync::Arc::new(crystal::PlaneScene {
            planes: planes,
            blockmap: bm,
        });
        world.insert(plane_scene.clone());

        let rad_scene = std::sync::Arc::new(crystal::Scene::new(world));
        world.insert(rad_scene.clone());

        std::thread::spawn(move || {
            loop {
                // let _pt = crystal::ProfTimer::new("rad update");
                rad_scene.do_rad();
            }
        });

        world.insert(light::LightWorker::new(plane_scene));
        println!("load done");
        Trans::Replace(Box::new(ExampleState::default()))
    }
}

struct ExampleState {
    scene: Option<Entity>,
}

impl Default for ExampleState {
    fn default() -> Self {
        ExampleState { scene: None }
    }
}

impl SimpleState for ExampleState {
    fn on_start(&mut self, data: StateData<'_, GameData<'_, '_>>) {
        let prefab_handle = data.world.exec(|loader: PrefabLoader<'_, MyPrefabData>| {
            loader.load("prefab/crystal_planes.ron", RonFormat, ())
        });
        let world = data.world;

        self.scene = Some(
            world
                .create_entity()
                .named("Crystal Planes Scene")
                .with(prefab_handle)
                .build(),
        );
    }
    fn handle_event(
        &mut self,
        data: StateData<'_, GameData<'_, '_>>,
        event: StateEvent,
    ) -> SimpleTrans {
        let StateData { world, .. } = data;
        let mut light_mode = WriteExpect::<LightMode>::fetch(world);

        if let StateEvent::Window(event) = &event {
            if is_key_down(&event, VirtualKeyCode::Escape) {
                let mut hide_cursor = world.write_resource::<HideCursor>();
                hide_cursor.hide = false;
            } else if is_mouse_button_down(&event, MouseButton::Left) {
                let mut hide_cursor = world.write_resource::<HideCursor>();
                hide_cursor.hide = true;
            } else if is_key_down(&event, VirtualKeyCode::Key1) {
                *light_mode = LightMode::RandomFlashing;
            } else if is_key_down(&event, VirtualKeyCode::Key2) {
                *light_mode = LightMode::Tron;
            } else if is_key_down(&event, VirtualKeyCode::Key3) {
                *light_mode = LightMode::LightSources;
            } else if is_key_down(&event, VirtualKeyCode::Key4) {
                *light_mode = LightMode::RendyLightSources;
            } else if is_key_down(&event, VirtualKeyCode::O) {
                add_animation(
                    world,
                    self.scene.unwrap(),
                    AnimationId::Translate,
                    0.25,
                    None,
                    false,
                );
            }
        }

        Trans::None
    }
}

fn main() -> Result<(), Error> {
    amethyst::start_logger(Default::default());

    let matches = App::new("crystal_planes")
        .version("1.0")
        .about("Realime Radiosity test")
        .arg(
            Arg::with_name("threads")
                .help("set number of rayon threads")
                .long("threads")
                .takes_value(true),
        )
        .get_matches();

    if let Some(threads) = matches.value_of("threads") {
        if let Ok(num_threads) = threads.parse::<usize>() {
            rayon::ThreadPoolBuilder::new()
                .num_threads(num_threads)
                .build_global()
                .unwrap();
        }
    }

    unsafe {
        // don't need / want denormals -> flush to zero
        core::arch::x86_64::_MM_SET_FLUSH_ZERO_MODE(core::arch::x86_64::_MM_FLUSH_ZERO_ON);
    }

    let app_root = application_root_dir()?;

    let assets_dir = app_root.join("src/assets");

    let display_config_path = app_root.join("src/config/display.ron");

    let key_bindings_path = app_root.join("src/config/input.ron");

    let game_data = GameDataBuilder::default()
        .with(AutoFovSystem::default(), "auto_fov", &[])
        .with_system_desc(PrefabLoaderSystemDesc::<MyPrefabData>::default(), "", &[])
        // .with_system_desc(quad::DiscoSystemDesc::default(), "disco_system", &[])
        .with_bundle(AnimationBundle::<AnimationId, Transform>::new(
            "animation_control_system",
            "sampler_interpolation_system",
        ))?
        .with_bundle(AnimationBundle::<AnimationId, Light>::new(
            "light_animation_control_system",
            "light_sampler_interpolation_system",
        ))?
        .with(
            // FIXME: create pausable system from SystemDesc?
            systems::RandomFlashingEmitSystem {}.pausable(LightMode::RandomFlashing),
            "random_flashing_emit_system",
            &[],
        )
        .with(
            systems::TronEmitSystem {}.pausable(LightMode::Tron),
            "tron_emit_system",
            &[],
        )
        .with(
            light::ApplyLightsSystem {}.pausable(LightMode::LightSources),
            "apply_lights_system",
            &[],
        )
        .with(
            light::ApplyRendyLightsSystem {}.pausable(LightMode::RendyLightSources),
            "apply_rendy_lights_system",
            &["light_sampler_interpolation_system"],
        )
        .with(
            systems::ApplyDiffuseColorSystem::default(),
            "apply_diffuse_color_system",
            &[],
        )
        .with_system_desc(
            systems::RunRadSceneSystemDesc::default(),
            "run_rad_system",
            &[
                "random_flashing_emit_system",
                "tron_emit_system",
                "apply_lights_system",
                "apply_rendy_lights_system",
                "apply_diffuse_color_system",
            ],
        )
        .with_system_desc(
            systems::CopyRadFrontSystemDesc::default(),
            "copy_rad_front_system",
            &["run_rad_system"],
        )
        .with_bundle(
            FlyControlBundle::<StringBindings>::new(
                Some(String::from("move_x")),
                Some(String::from("move_y")),
                Some(String::from("move_z")),
            )
            .with_sensitivity(0.1, 0.1)
            .with_speed(10.0),
        )?
        .with_bundle(
            TransformBundle::new().with_dep(&["fly_movement", "sampler_interpolation_system"]),
        )?
        .with_bundle(
            InputBundle::<StringBindings>::new().with_bindings_from_file(&key_bindings_path)?,
        )?
        .with_bundle(
            RenderingBundle::<DefaultBackend>::new()
                .with_plugin(
                    RenderToWindow::from_config_path(display_config_path)?
                        .with_clear([0.34, 0.36, 0.52, 1.0]),
                )
                .with_plugin(RenderShaded3D::default())
                // Add our custom render plugin to the rendering bundle.
                .with_plugin(RenderQuad::default())
                .with_plugin(RenderSkybox::with_colors(
                    Srgb::new(0.82, 0.51, 0.50),
                    Srgb::new(0.18, 0.11, 0.85),
                )),
        )?;

    let mut game = Application::build(assets_dir, MapLoadState)?.build(game_data)?;
    game.run();
    Ok(())
}

fn add_animation(
    world: &World,
    entity: Entity,
    id: AnimationId,
    rate: f32,
    defer: Option<(AnimationId, DeferStartRelation)>,
    toggle_if_exists: bool,
) {
    let anim_set_storage = world.read_storage::<AnimationSet<AnimationId, Transform>>();
    let anim_set = anim_set_storage.get(entity);
    println!("anim_set: {:?}", anim_set);
    let animation = anim_set
        .expect("missing AnimationSet")
        .get(&id)
        .cloned()
        .unwrap();

    // let animation = world
    //     .read_storage::<AnimationSet<AnimationId, Transform>>()
    //     .get(entity)
    //     .and_then(|s| s.get(&id))
    //     .cloned()
    //     .unwrap();
    let mut sets = world.write_storage();
    let control_set = get_animation_set::<AnimationId, Transform>(&mut sets, entity).unwrap();
    match defer.clone() {
        None => {
            if toggle_if_exists && control_set.has_animation(id) {
                control_set.toggle(id);
            } else {
                control_set.add_animation(
                    id,
                    &animation,
                    EndControl::Loop(None),
                    rate,
                    AnimationCommand::Start,
                );
            }
        }

        Some((defer_id, defer_relation)) => {
            control_set.add_deferred_animation(
                id,
                &animation,
                EndControl::Normal,
                rate,
                AnimationCommand::Start,
                defer_id,
                defer_relation,
            );
        }
    }

    let mut sets = world.write_storage();
    let anim_set_storage = world.read_storage::<AnimationSet<AnimationId, Light>>();
    let anim_set = anim_set_storage.get(entity);
    println!("anim_set: {:?}", anim_set);
    let animation = anim_set
        .expect("missing AnimationSet")
        .get(&id)
        .cloned()
        .unwrap();
    let control_set = get_animation_set::<AnimationId, Light>(&mut sets, entity).unwrap();
    match defer {
        None => {
            if toggle_if_exists && control_set.has_animation(id) {
                control_set.toggle(id);
            } else {
                control_set.add_animation(
                    id,
                    &animation,
                    EndControl::Loop(None),
                    rate,
                    AnimationCommand::Start,
                );
            }
        }

        Some((defer_id, defer_relation)) => {
            control_set.add_deferred_animation(
                id,
                &animation,
                EndControl::Normal,
                rate,
                AnimationCommand::Start,
                defer_id,
                defer_relation,
            );
        }
    }
}
