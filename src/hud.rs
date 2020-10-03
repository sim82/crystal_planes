use bevy::{
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin},
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
fn update_hud_system(
    diagnostics: Res<Diagnostics>,
    render_status: Res<RenderStatus>,
    mut query: Query<&mut Text>,
) {
    for mut text in &mut query.iter() {
        if let Some(fps) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
            if let Some(average) = fps.average() {
                text.value = format!("FPS: {:.2}\nrender status: {}", average, render_status.text);
            }
        }
    }
}

fn setup_hud_system(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font_handle = asset_server
        .load("assets/fonts/FiraMono-Medium.ttf")
        .unwrap();
    commands
        // 2d camera
        .spawn(UiCameraComponents::default())
        // texture
        .spawn(TextComponents {
            style: Style {
                align_self: AlignSelf::FlexEnd,
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
