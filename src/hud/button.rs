use bevy::prelude::*;

use crate::property::PropertyRegistry;

/// This example illustrates how to create a button that changes color and text based on its interaction state.

pub struct ButtonPlugin;
impl Plugin for ButtonPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<ButtonMaterials>()
            .add_system(toggle_button_system.system());
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
    // action: Box<dyn Fn(&World) + Send + Sync>,
    // get_label: Box<dyn Fn(&World) -> String + Send + Sync>,
    pub property_name: String,
    pub on_text: String,
    pub off_text: String,
}

// impl ToggleButton {
//     // pub fn new<A, G>(action: A, get_label: G) -> ToggleButton
//     // where
//     //     A: Fn(&World) + 'static + Send + Sync,
//     //     G: Fn(&World) -> String + 'static + Send + Sync,
//     // {
//     //     ToggleButton {
//     //         action: Box::new(action),
//     //         get_label: Box::new(get_label),
//     //     }
//     // }
// }

// fn toggle_button_system(world: &mut World) {
//     let mut query = world.query::<(
//         &ToggleButton,
//         &Children,
//         &Interaction,
//         &Handle<ColorMaterial>,
//     )>();
//     for (toggle_button, _children, interaction, material) in query.iter(world) {
//         match *interaction {
//             Interaction::Clicked => {
//                 // {
//                 //     let action = &*toggle_button.action;
//                 //     action(world);
//                 // }
//                 let _button_materials = world.get_resource::<ButtonMaterials>().unwrap();
//                 // *material = button_materials.pressed.clone();
//             }
//             Interaction::Hovered => {
//                 let _button_materials = world.get_resource::<ButtonMaterials>().unwrap();
//                 // *material = button_materials.hovered.clone();
//             }
//             Interaction::None => {
//                 let button_materials = world.get_resource::<ButtonMaterials>().unwrap();
//                 *material = button_materials.normal.clone();
//             }
//         }
//     }
// }

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
        Changed<Interaction>,
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
                println!("clicked: {:?}", is_on);
                text.sections[0].value = if is_on {
                    &toggle_button.on_text
                } else {
                    &toggle_button.off_text
                }
                .clone();
                *material = button_materials.pressed.clone();
            }
            Interaction::Hovered => {
                text.sections[0].value = "Hover".to_string();
                *material = button_materials.hovered.clone();
            }
            Interaction::None => {
                text.sections[0].value = "Button".to_string();
                *material = button_materials.normal.clone();
            }
        }
    }
}

// fn toggle_button_text_system(world: &mut World) {
//     let mut set_texts = Vec::new();
//     let mut query = world.query::<(&ToggleButton, &Children)>();

//     for (toggle_button, children) in query.iter(world) {
//         let get_label = &*toggle_button.get_label;
//         set_texts.push((children[0], get_label(world)));
//     }

//     // cannot mutate Text while iterating over query
//     for (ent, s) in set_texts.drain(..) {
//         let mut text = world.get_mut::<Text>(ent).unwrap();
//         // text.value = s;
//         text.sections[0].value = s;
//     }
// }
