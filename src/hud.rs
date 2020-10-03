use bevy::{
    diagnostic::{DiagnosticId, Diagnostics, FrameTimeDiagnosticsPlugin},
    prelude::*,
};

/// This example illustrates how to create text and update it in a system. It displays the current FPS in the upper left hand corner.
pub struct RenderStatus {
    pub text: String,
}

impl Default for RenderStatus {
    fn default() -> Self {
        RenderStatus {
            text: "unknown".into(),
        }
    }
}

pub enum HudSrc {
    Diagnostics(String, DiagnosticId, bool),
    RenderStatus,
}
fn mag_to_str(mag: i32) -> &'static str {
    match mag {
        0 => "",
        1 => "K",
        2 => "M",
        3 => "G",
        4 => "T",
        5 => "P",
        6 => "E",
        _ => "too large",
    }
}
fn update_hud_system(
    diagnostics: Res<Diagnostics>,
    render_status: Res<RenderStatus>,
    mut query: Query<(&mut Text, &HudSrc)>,
) {
    for (mut text, src) in &mut query.iter() {
        match src {
            HudSrc::Diagnostics(diag_text, id, unit) => {
                if let Some(fps) = diagnostics.get(*id) {
                    if let Some(mut average) = fps.average() {
                        if *unit {
                            let mut mag = 0;
                            while average >= 1000f64 {
                                average /= 1000f64;
                                mag += 1;
                            }
                            text.value = format!("{} {:.3}{}", diag_text, average, mag_to_str(mag))
                        } else {
                            text.value = format!("{} {:.2}", diag_text, average);
                        }
                    }
                }
            }
            HudSrc::RenderStatus => (text.value = format!("render status: {}", render_status.text)),
        }

        // if let Some(fps) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
        //     if let Some(average) = fps.average() {
        //         text.value = format!("FPS: {:.2}\nrender status: {}", average, render_status.text);
        //     }
        // }
    }
}

fn setup_hud_system(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let font_handle = asset_server
        .load("assets/fonts/FiraMono-Medium.ttf")
        .unwrap();
    commands
        // 2d camera
        .spawn(UiCameraComponents::default())
        .spawn(NodeComponents {
            style: Style {
                flex_direction: FlexDirection::ColumnReverse,
                flex_shrink: 1f32,
                ..Default::default()
            },
            mesh: Handle::default(), // meh, is this the right way to get an invisible flex node?
            ..Default::default()
        })
        .with_children(|parent| {
            parent
                .spawn(TextComponents {
                    style: Style {
                        align_self: AlignSelf::FlexStart,
                        // margin: Rect {
                        //     right: Val::Px(64f32),
                        //     ..Default::default()
                        // },
                        ..Default::default()
                    },
                    text: Text {
                        value: "FPS:".to_string(),
                        font: font_handle,
                        style: TextStyle {
                            font_size: 24.0,
                            color: Color::WHITE,
                        },
                    },
                    ..Default::default()
                })
                .with(HudSrc::Diagnostics(
                    "FPS".into(),
                    FrameTimeDiagnosticsPlugin::FPS,
                    false,
                ))
                .spawn(TextComponents {
                    style: Style {
                        align_self: AlignSelf::FlexStart,
                        // margin: Rect {
                        //     right: Val::Px(64f32),
                        //     ..Default::default()
                        // },
                        ..Default::default()
                    },
                    text: Text {
                        value: "Int/s:".to_string(),
                        font: font_handle,
                        style: TextStyle {
                            font_size: 24.0,
                            color: Color::WHITE,
                        },
                    },
                    ..Default::default()
                })
                .with(HudSrc::Diagnostics(
                    "Int/s".into(),
                    super::quad_render::RAD_INT_PER_SECOND,
                    true,
                ))
                .spawn(TextComponents {
                    style: Style {
                        align_self: AlignSelf::FlexStart,
                        // margin: Rect {
                        //     right: Val::Px(256f32),
                        //     ..Default::default()
                        // },
                        ..Default::default()
                    },
                    text: Text {
                        value: "Int/s:".to_string(),
                        font: font_handle,
                        style: TextStyle {
                            font_size: 24.0,
                            color: Color::WHITE,
                        },
                    },
                    ..Default::default()
                })
                .with(HudSrc::RenderStatus);
        });
}

#[derive(Default)]
pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_startup_system(setup_hud_system.system())
            .add_system(update_hud_system.system())
            .init_resource::<RenderStatus>();
    }
}
