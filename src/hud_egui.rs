use bevy::{
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin},
    prelude::*,
};
use bevy_egui::{egui, EguiContext};

use crate::{
    hud::{HudElement, HudSrc, RenderStatus, RAD_INT_PER_SECOND},
    property::{PropertyName, PropertyRegistry, PropertyUpdateEvent, PropertyValue},
};

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

pub fn hud_egui_setup_system(mut commands: Commands) {
    commands
        .spawn()
        .insert(HudElement::TextWithSource(HudSrc::Diagnostics(
            "FPS".into(),
            FrameTimeDiagnosticsPlugin::FPS,
            false,
        )));
    commands
        .spawn()
        .insert(HudElement::TextWithSource(HudSrc::Diagnostics(
            "Int/s".into(),
            RAD_INT_PER_SECOND,
            true,
        )));
    commands
        .spawn()
        .insert(HudElement::TextWithSource(HudSrc::RenderStatus));
}

pub struct StringEdit {
    current_string: String,
}

pub fn hud_egui_system(
    mut commands: Commands,
    egui_context: Res<EguiContext>,
    property_registry: Res<PropertyRegistry>,
    mut property_update_events: EventWriter<PropertyUpdateEvent>,
    property_query: Query<(&PropertyValue, &PropertyName)>,
    diagnostics: Res<Diagnostics>,
    render_status: Res<RenderStatus>,
    hud_elements_query: Query<(Entity, &HudOrder, &HudElement)>,
    mut string_edit_query: Query<&mut StringEdit>,
) {
    egui::Window::new("HUD").show(egui_context.ctx(), |ui| {
        let mut ordered: Vec<_> = hud_elements_query.iter().collect();
        ordered.sort_by_key(|(_, o, _)| *o);

        for (entity, _, element) in ordered {
            match element {
                HudElement::TextWithSource(s) => {
                    let text = match s {
                        HudSrc::Diagnostics(diag_text, id, unit) => {
                            if let Some(fps) = diagnostics.get(*id) {
                                let mut average = fps.average().unwrap_or_default();
                                if *unit {
                                    let mut mag = 0;
                                    while average >= 1000f64 {
                                        average /= 1000f64;
                                        mag += 1;
                                    }

                                    format!("{} {:.3}{}", diag_text, average, mag_to_str(mag))
                                } else {
                                    format!("{} {:.2}", diag_text, average)
                                }
                            } else {
                                format!("failed: {:?}", id)
                            }
                        }
                        HudSrc::RenderStatus => {
                            format!("render status: {}", render_status.text)
                        }
                        HudSrc::LoadingScreen => String::new(),
                    };
                    ui.label(text);
                }
                HudElement::ToggleButtonProperty(property_name, _on_text, _off_text) => {
                    match property_registry.get(&property_name) {
                        Some(rs) => {
                            let (v, _) = property_query.get(rs).unwrap();
                            let v = match v {
                                PropertyValue::Bool(v) => *v,
                                _ => false,
                            };
                            if ui.button(format!("{}:{:?}", property_name, v)).clicked() {
                                property_update_events.send(PropertyUpdateEvent::new(
                                    property_name.clone(),
                                    PropertyValue::Bool(!v),
                                ));
                            }
                        }
                        _ => {
                            ui.label(format!("failed: {}", property_name));
                        }
                    }
                }
                HudElement::EditThis => match property_query.get(entity) {
                    Ok((property_value, property_name)) => {
                        match property_value {
                            PropertyValue::Bool(v) => {
                                if ui.button(format!("{}:{:?}", property_name.0, v)).clicked() {
                                    property_update_events.send(PropertyUpdateEvent::new(
                                        property_name.0.clone(),
                                        PropertyValue::Bool(!v),
                                    ));
                                }
                            }
                            PropertyValue::String(s) => match string_edit_query.get_mut(entity) {
                                Ok(mut string_edit) => {
                                    if ui
                                        .text_edit_singleline(&mut string_edit.current_string)
                                        .lost_focus()
                                    {
                                        commands.entity(entity).remove::<StringEdit>();
                                        property_update_events.send(PropertyUpdateEvent::new(
                                            property_name.0.clone(),
                                            PropertyValue::String(
                                                string_edit.current_string.clone(),
                                            ),
                                        ));
                                    }
                                }
                                _ => {
                                    if ui.button(&property_name.0).clicked() {
                                        commands.entity(entity).insert(StringEdit {
                                            current_string: s.to_string(),
                                        });
                                    }
                                }
                            },
                            _ => (),
                        };
                    }
                    _ => {
                        ui.label(format!("failed: {:?}", entity));
                    }
                },
            }
        }

        // ui.add(egui::TextEdit::singleline("text").hint_text("Write something here"));
    });
}

#[derive(Copy, Clone, Eq, PartialEq, Default, Ord, PartialOrd)]
pub struct HudOrder {
    id: usize,
}

impl HudOrder {
    pub fn next(&mut self) -> HudOrder {
        let ret = *self;
        self.id += 1;
        ret
    }
}

#[derive(Default)]
pub struct HudEguiPlugin;

impl Plugin for HudEguiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RenderStatus>()
            .init_resource::<HudOrder>()
            .add_startup_system(hud_egui_setup_system.system())
            .add_system(hud_egui_system.system());
    }
}
