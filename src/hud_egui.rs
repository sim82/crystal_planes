use bevy::{
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin},
    prelude::*,
};
use bevy_egui::{egui, EguiContext};

use crate::{
    hud::{HudElement, HudSrc, RenderStatus, RAD_INT_PER_SECOND},
    propent::{PropentRegistry, PropertyUpdateEvent, PropertyValue},
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

pub fn hud_egui_system(
    egui_context: Res<EguiContext>,
    propent_registry: Res<PropentRegistry>,
    mut property_update_events: EventWriter<PropertyUpdateEvent>,
    propent_query: Query<&PropertyValue>,
    diagnostics: Res<Diagnostics>,
    render_status: Res<RenderStatus>,
) {
    let hud_elements = [
        HudElement::TextWithSource(HudSrc::Diagnostics(
            "FPS".into(),
            FrameTimeDiagnosticsPlugin::FPS,
            false,
        )),
        HudElement::TextWithSource(HudSrc::Diagnostics(
            "Int/s".into(),
            RAD_INT_PER_SECOND,
            true,
        )),
        HudElement::TextWithSource(HudSrc::RenderStatus),
        HudElement::ToggleButtonPropent(
            "rotator_system.enabled".to_string(),
            "Stop".to_string(),
            "Start".to_string(),
        ),
        HudElement::ToggleButtonPropent(
            "demo_system.light_enabled".to_string(),
            "disable light".to_string(),
            "enable light".to_string(),
        ),
        HudElement::ToggleButtonPropent(
            "demo_system.cycle".to_string(),
            "disable cycle".to_string(),
            "enable cycle".to_string(),
        ),
    ];

    egui::Window::new("HUD").show(egui_context.ctx(), |ui| {
        for element in hud_elements {
            match element {
                HudElement::TextWithSource(s) => {
                    let text = match s {
                        HudSrc::Diagnostics(diag_text, id, unit) => {
                            if let Some(fps) = diagnostics.get(id) {
                                if let Some(mut average) = fps.average() {
                                    if unit {
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
                                    String::new()
                                }
                            } else {
                                String::new()
                            }
                        }
                        HudSrc::RenderStatus => {
                            format!("render status: {}", render_status.text)
                        }
                        HudSrc::LoadingScreen => String::new(),
                    };
                    ui.label(text);
                }
                HudElement::ToggleButtonPropent(property_name, on_text, off_text) => {
                    ui.button(property_name);
                }
            }
        }
    });

    egui::Window::new("Hello").show(egui_context.ctx(), |ui| {
        ui.label("world");
        match propent_registry.get("rotator_system.enabled") {
            Some(rs) => {
                let v = propent_query.get(rs).unwrap();
                let v = match v {
                    PropertyValue::Bool(v) => *v,
                    _ => false,
                };
                if ui.button(format!("rotate: {:?}", v)).clicked() {
                    info!("quit");
                    property_update_events.send(PropertyUpdateEvent::new(
                        "rotator_system.enabled".to_string(),
                        PropertyValue::Bool(!v),
                    ));
                }
            }
            _ => (),
        }
    });
}
