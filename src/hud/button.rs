use bevy::prelude::*;

/// This example illustrates how to create a button that changes color and text based on its interaction state.

pub struct ButtonPlugin;
impl Plugin for ButtonPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<ButtonMaterials>()
            .add_system(toggle_button_system.exclusive_system())
            .add_system(toggle_button_text_system.exclusive_system());
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
    action: Box<dyn FnMut(&mut World) -> () + Send + Sync>,
    get_label: Box<dyn Fn(&mut World) -> String + Send + Sync>,
}

impl ToggleButton {
    pub fn new<A, G>(action: A, get_label: G) -> ToggleButton
    where
        A: FnMut(&mut World) -> () + 'static + Send + Sync,
        G: Fn(&mut World) -> String + 'static + Send + Sync,
    {
        ToggleButton {
            action: Box::new(action),
            get_label: Box::new(get_label),
        }
    }
}

fn toggle_button_system(world: &mut World) {
    // let mut query = world.query::<(
    //     &mut ToggleButton,
    //     &Children,
    //     &Interaction,
    //     &mut Handle<ColorMaterial>,
    // )>();
    // for (mut toggle_button, _children, interaction, mut material) in query.iter(world) {
    //     match *interaction {
    //         Interaction::Clicked => {
    //             {
    //                 let action = &mut *toggle_button.action;
    //                 action(world);
    //             }
    //             let button_materials = world.get_resource::<ButtonMaterials>().unwrap();
    //             *material = button_materials.pressed.clone();
    //         }
    //         Interaction::Hovered => {
    //             let button_materials = world.get_resource::<ButtonMaterials>().unwrap();
    //             *material = button_materials.hovered.clone();
    //         }
    //         Interaction::None => {
    //             let button_materials = world.get_resource::<ButtonMaterials>().unwrap();
    //             *material = button_materials.normal.clone();
    //         }
    //     }
    // }
}

// fn toggle_button_system(
//     button_materials: Res<ButtonMaterials>,
//     query: Query<(
//         &mut ToggleButton,
//         &Children,
//         Mutated<Interaction>,
//         &mut Handle<ColorMaterial>,
//     )>,
// ) {
//     for (mut toggle_button, _children, interaction, mut material) in query {
//         match *interaction {
//             Interaction::Clicked => {
//                 {
//                     let action = &mut *toggle_button.action;
//                     action(res);
//                 }
//                 let button_materials = res.get::<ButtonMaterials>().unwrap();
//                 *material = button_materials.pressed.clone();
//             }
//             Interaction::Hovered => {
//                 let button_materials = res.get::<ButtonMaterials>().unwrap();
//                 *material = button_materials.hovered.clone();
//             }
//             Interaction::None => {
//                 let button_materials = res.get::<ButtonMaterials>().unwrap();
//                 *material = button_materials.normal.clone();
//             }
//         }
//     }
// }

fn toggle_button_text_system(world: &mut World) {
    // let mut set_texts = Vec::new();
    // let query = world.query::<(&ToggleButton, &Children)>();

    // for (toggle_button, children) in query {
    //     let get_label = &*toggle_button.get_label;
    //     set_texts.push((children[0], get_label(world)));
    // }

    // // cannot mutate Text while iterating over query
    // for (ent, s) in set_texts.drain(..) {
    //     let mut text = world.get_mut::<Text>(ent).unwrap();
    //     text.value = s;
    // }
}
