use bevy::prelude::*;

use crate::{
    propent::{self, PropertyAccess, PropertyName, PropertyUpdateEvent},
    property::{PropertyRegistry, PropertyValue},
};

/// This example illustrates how to create a button that changes color and text based on its interaction state.

pub struct ButtonPlugin;
impl Plugin for ButtonPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<ButtonMaterials>()
            .add_system(toggle_button_system.system())
            .add_system(propent_toggle_button_system.system());
        // .add_system(toggle_button_text_system.exclusive_system());
    }
}

pub struct ButtonMaterials {
    pub normal: Handle<ColorMaterial>,
    pub hovered: Handle<ColorMaterial>,
    pub pressed: Handle<ColorMaterial>,
}

impl FromWorld for ButtonMaterials {
    fn from_world(world: &mut World) -> Self {
        let mut materials = world.get_resource_mut::<Assets<ColorMaterial>>().unwrap();
        ButtonMaterials {
            normal: materials.add(Color::rgb(0.02, 0.02, 0.02).into()),
            hovered: materials.add(Color::rgb(0.05, 0.05, 0.05).into()),
            pressed: materials.add(Color::rgb(0.1, 0.5, 0.1).into()),
        }
    }
}

pub struct ToggleButton {
    pub property_name: String,
    pub on_text: String,
    pub off_text: String,
}

fn toggle_button_system(
    button_materials: Res<ButtonMaterials>,
    mut property_registry: ResMut<PropertyRegistry>,
    mut interaction_query: Query<
        (
            &Interaction,
            &mut Handle<ColorMaterial>,
            &Children,
            &ToggleButton,
        ),
        (Changed<Interaction>, Without<PropertyAccess>),
    >,
    mut text_query: Query<&mut Text>,
) {
    for (interaction, mut material, children, toggle_button) in interaction_query.iter_mut() {
        let mut text = text_query.get_mut(children[0]).unwrap();
        match *interaction {
            Interaction::Clicked => {
                let is_on =
                    if let Some(v) = property_registry.get_bool_mut(&toggle_button.property_name) {
                        *v = !*v;
                        *v
                    } else {
                        false
                    };
                text.sections[0].value = if is_on {
                    &toggle_button.on_text
                } else {
                    &toggle_button.off_text
                }
                .clone();
                *material = button_materials.pressed.clone();
            }
            Interaction::Hovered => {
                *material = button_materials.hovered.clone();
            }
            Interaction::None => {
                if let Some(is_on) = property_registry.get_bool(&toggle_button.property_name) {
                    text.sections[0].value = if *is_on {
                        &toggle_button.on_text
                    } else {
                        &toggle_button.off_text
                    }
                    .clone();
                }
                *material = button_materials.normal.clone();
            }
        }
    }
}

fn propent_toggle_button_system(
    button_materials: Res<ButtonMaterials>,
    mut property_update_events: EventWriter<propent::PropertyUpdateEvent>,
    query: Query<(&PropertyAccess, &ToggleButton, &Children), Changed<PropertyAccess>>,
    mut interaction_query: Query<
        (
            &Interaction,
            &mut Handle<ColorMaterial>,
            &Children,
            &ToggleButton,
            &PropertyName,
            &PropertyAccess,
        ),
        (Changed<Interaction>,),
    >,
    mut text_query: Query<&mut Text>,
) {
    for (interaction, mut material, children, toggle_button, property_name, property_access) in
        interaction_query.iter_mut()
    {
        let mut text = text_query.get_mut(children[0]).unwrap();
        match *interaction {
            Interaction::Clicked => {
                let v = if let PropertyValue::Bool(v) = property_access.cache {
                    v
                } else {
                    false
                };
                let new_value = PropertyValue::Bool(!v);
                println!("send update: {} {:?}", property_name.0, new_value);
                property_update_events
                    .send(PropertyUpdateEvent::new(property_name.0.clone(), new_value));
                *material = button_materials.pressed.clone();
            }
            Interaction::Hovered => {
                *material = button_materials.hovered.clone();
            }
            Interaction::None => {
                *material = button_materials.normal.clone();
            }
        }
    }

    for (access, toggle_button, children) in query.iter() {
        println!("change detected: {:?}", access.cache);
        let mut text = text_query.get_mut(children[0]).unwrap();

        let is_on = if let PropertyValue::Bool(v) = access.cache {
            v
        } else {
            false
        };
        text.sections[0].value = if is_on {
            &toggle_button.on_text
        } else {
            &toggle_button.off_text
        }
        .clone();
    }
}
