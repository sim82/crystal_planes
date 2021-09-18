use bevy::{
    diagnostic::{DiagnosticId, Diagnostics, FrameTimeDiagnosticsPlugin},
    prelude::*,
};

use crate::property::{PropertyAccess, PropertyName};
pub const RAD_INT_PER_SECOND: DiagnosticId =
    DiagnosticId::from_u128(337040787172757619024841343456040760896);

// mod button;

// // FIXME: only defined here because hud code directly modifies it. Implementation should be moved from main.rs
pub struct DemoSystemState {
    pub cycle_timer: Timer,
}

impl Default for DemoSystemState {
    fn default() -> Self {
        DemoSystemState {
            cycle_timer: Timer::from_seconds(1f32, true),
        }
    }
}

// /// This example illustrates how to create text and update it in a system. It displays the current FPS in the upper left hand corner.
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

// #[allow(dead_code)]
#[derive(Clone)]
pub enum HudSrc {
    Diagnostics(String, DiagnosticId, bool),
    RenderStatus,
    LoadingScreen,
}
// fn mag_to_str(mag: i32) -> &'static str {
//     match mag {
//         0 => "",
//         1 => "K",
//         2 => "M",
//         3 => "G",
//         4 => "T",
//         5 => "P",
//         6 => "E",
//         _ => "too large",
//     }
// }
// fn update_hud_system(
//     diagnostics: Res<Diagnostics>,
//     render_status: Res<RenderStatus>,
//     mut query: Query<(&mut Text, &HudSrc)>,
// ) {
//     for (mut text, src) in query.iter_mut() {
//         match src {
//             HudSrc::Diagnostics(diag_text, id, unit) => {
//                 if let Some(fps) = diagnostics.get(*id) {
//                     if let Some(mut average) = fps.average() {
//                         if *unit {
//                             let mut mag = 0;
//                             while average >= 1000f64 {
//                                 average /= 1000f64;
//                                 mag += 1;
//                             }
//                             text.sections[0].value =
//                                 format!("{} {:.3}{}", diag_text, average, mag_to_str(mag))
//                         } else {
//                             text.sections[0].value = format!("{} {:.2}", diag_text, average);
//                         }
//                     }
//                 }
//             }
//             HudSrc::RenderStatus => {
//                 text.sections[0].value = format!("render status: {}", render_status.text)
//             }
//             HudSrc::LoadingScreen => (),
//         }

//         // if let Some(fps) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
//         //     if let Some(average) = fps.average() {
//         //         text.value = format!("FPS: {:.2}\nrender status: {}", average, render_status.text);
//         //     }
//         // }
//     }
// }

#[derive(Clone)]
pub enum HudElement {
    TextWithSource(HudSrc),
    ToggleButtonPropent(String, String, String),
    ToggleThis,
}

// fn build_children(
//     parent: &mut ChildBuilder,
//     children: &[HudElement],
//     asset_server: Res<AssetServer>,
//     button_materials: Res<button::ButtonMaterials>,
// ) {
//     let font_handle = asset_server.load("fonts/FiraMono-Medium.ttf");
//     for child in children {
//         match child {
//             HudElement::TextWithSource(hud_src) => {
//                 parent
//                     .spawn_bundle(TextBundle {
//                         style: Style {
//                             align_self: AlignSelf::FlexStart,
//                             ..Default::default()
//                         },
//                         text: Text::with_section(
//                             "<unknown>".to_string(),
//                             TextStyle {
//                                 font: font_handle.clone(),
//                                 font_size: 20.0,
//                                 color: Color::WHITE,
//                             },
//                             TextAlignment::default(),
//                         ),
//                         ..Default::default()
//                     })
//                     .insert(hud_src.clone());
//             }
//             HudElement::ToggleButtonPropent(property_name, on_text, off_text) => {
//                 parent
//                     .spawn_bundle(ButtonBundle {
//                         style: Style {
//                             size: Size::new(Val::Px(120.0), Val::Px(40.0)),
//                             justify_content: JustifyContent::Center,
//                             align_items: AlignItems::Center,
//                             align_self: AlignSelf::FlexStart,
//                             ..Default::default()
//                         },
//                         material: button_materials.normal.clone(),
//                         ..Default::default()
//                     })
//                     .insert(PropertyName(property_name.clone()))
//                     .insert(PropertyAccess::default())
//                     .insert(button::ToggleButton {
//                         property_name: property_name.clone(),
//                         on_text: on_text.clone(),
//                         off_text: off_text.clone(),
//                     })
//                     .with_children(|parent| {
//                         parent.spawn_bundle(TextBundle {
//                             text: Text::with_section(
//                                 off_text.clone(),
//                                 TextStyle {
//                                     font: asset_server.load("fonts/FiraSans-Bold.ttf"),

//                                     font_size: 20.0,
//                                     color: Color::rgb(0.8, 0.8, 0.8),
//                                 },
//                                 TextAlignment::default(),
//                             ),
//                             ..Default::default()
//                         });
//                     });
//             }
//         }
//     }
// }

// fn setup_hud_system2(
//     mut commands: Commands,
//     asset_server: Res<AssetServer>,
//     mut _materials: ResMut<Assets<ColorMaterial>>,
//     button_materials: Res<button::ButtonMaterials>,
// ) {
//     let hud_elements = [
//         HudElement::TextWithSource(HudSrc::Diagnostics(
//             "FPS".into(),
//             FrameTimeDiagnosticsPlugin::FPS,
//             false,
//         )),
//         HudElement::TextWithSource(HudSrc::Diagnostics(
//             "Int/s".into(),
//             RAD_INT_PER_SECOND,
//             true,
//         )),
//         HudElement::TextWithSource(HudSrc::RenderStatus),
//         HudElement::ToggleButtonPropent(
//             "rotator_system.enabled".to_string(),
//             "Stop".to_string(),
//             "Start".to_string(),
//         ),
//         HudElement::ToggleButtonPropent(
//             "demo_system.light_enabled".to_string(),
//             "disable light".to_string(),
//             "enable light".to_string(),
//         ),
//         HudElement::ToggleButtonPropent(
//             "demo_system.cycle".to_string(),
//             "disable cycle".to_string(),
//             "enable cycle".to_string(),
//         ),
//     ];

//     commands
//         // 2d camera
//         .spawn_bundle(UiCameraBundle::default());
//     commands
//         .spawn_bundle(NodeBundle {
//             style: Style {
//                 flex_direction: FlexDirection::ColumnReverse,
//                 flex_shrink: 1f32,
//                 ..Default::default()
//             },
//             mesh: Handle::default(), // meh, is this the right way to get an invisible flex node?
//             ..Default::default()
//         })
//         .with_children(|parent| {
//             build_children(parent, &hud_elements, asset_server, button_materials)
//         });
// }

// #[derive(Default)]
// pub struct HudPlugin;

// impl Plugin for HudPlugin {
//     fn build(&self, app: &mut App) {
//         app.init_resource::<RenderStatus>()
//             .add_startup_system(setup_hud_system2.system())
//             .add_system(update_hud_system.system())
//             .add_plugin(button::ButtonPlugin);
//     }
// }
