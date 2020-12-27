use bevy::{app::ScheduleRunnerPlugin, app::ScheduleRunnerSettings, prelude::*};
use std::time::Duration;

fn main() {
    // App::build()
    //     .add_default_plugins()
    //     .add_plugin(FrameTimeDiagnosticsPlugin::default())
    //     .add_plugin(PrintDiagnosticsPlugin::default())
    //     .add_startup_system(setup.system())

    //     .run();

    App::build()
        .add_resource(ScheduleRunnerSettings::run_loop(Duration::from_secs_f64(
            1.0 / 60.0,
        )))
        .add_plugin(ScheduleRunnerPlugin::default())
        .add_system(system1.system())
        .add_system(system2.system())
        .init_resource::<Res1>()
        .run();
}

#[derive(Default)]
struct Res1();
fn system1(_res1: Res<Res1>) {
    println!("system1 begin {:?}", std::thread::current().id());
    std::thread::sleep(std::time::Duration::from_secs(1));
    println!("system1 end");
}

fn system2(_res1: Res<Res1>) {
    println!("system2 begin {:?}", std::thread::current().id());
    std::thread::sleep(std::time::Duration::from_secs(1));

    println!("system2 end");
}
