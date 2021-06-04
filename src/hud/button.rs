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
    action: Box<dyn Fn(&World) + Send + Sync>,
    get_label: Box<dyn Fn(&World) -> String + Send + Sync>,
}

impl ToggleButton {
    pub fn new<A, G>(action: A, get_label: G) -> ToggleButton
    where
        A: Fn(&World) + 'static + Send + Sync,
        G: Fn(&World) -> String + 'static + Send + Sync,
    {
        ToggleButton {
            action: Box::new(action),
            get_label: Box::new(get_label),
        }
    }
}

fn toggle_button_system(world: &mut World) {
    let mut query = world.query::<(
        &ToggleButton,
        &Children,
        &Interaction,
        &Handle<ColorMaterial>,
    )>();
    for (toggle_button, _children, interaction, _material) in query.iter(world) {
        match *interaction {
            Interaction::Clicked => {
                {
                    let action = &*toggle_button.action;
                    action(world);
                }
                let _button_materials = world.get_resource::<ButtonMaterials>().unwrap();
                // *material = button_materials.pressed.clone();
            }
            Interaction::Hovered => {
                let _button_materials = world.get_resource::<ButtonMaterials>().unwrap();
                // *material = button_materials.hovered.clone();
            }
            Interaction::None => {
                let _button_materials = world.get_resource::<ButtonMaterials>().unwrap();
                // *material = button_materials.normal.clone();
            }
        }
    }
}

fn toggle_button_text_system(world: &mut World) {
    let mut set_texts = Vec::new();
    let mut query = world.query::<(&ToggleButton, &Children)>();

    for (toggle_button, children) in query.iter(world) {
        let get_label = &*toggle_button.get_label;
        set_texts.push((children[0], get_label(world)));
    }

    // cannot mutate Text while iterating over query
    for (ent, s) in set_texts.drain(..) {
        let mut text = world.get_mut::<Text>(ent).unwrap();
        // text.value = s;
        text.sections[0].value = s;
    }
}
