use bevy::prelude::*;

/// This example illustrates how to create a button that changes color and text based on its interaction state.

pub struct ButtonPlugin;
impl Plugin for ButtonPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<ButtonMaterials>()
            .add_system(toggle_button_system.thread_local_system())
            .add_system(toggle_button_text_system.thread_local_system());
    }
}

pub struct ButtonMaterials {
    pub normal: Handle<ColorMaterial>,
    pub hovered: Handle<ColorMaterial>,
    pub pressed: Handle<ColorMaterial>,
}

impl FromResources for ButtonMaterials {
    fn from_resources(resources: &Resources) -> Self {
        let mut materials = resources.get_mut::<Assets<ColorMaterial>>().unwrap();
        ButtonMaterials {
            normal: materials.add(Color::rgb(0.02, 0.02, 0.02).into()),
            hovered: materials.add(Color::rgb(0.05, 0.05, 0.05).into()),
            pressed: materials.add(Color::rgb(0.1, 0.5, 0.1).into()),
        }
    }
}

pub struct ToggleButton {
    action: Box<dyn FnMut(&mut Resources) -> () + Send + Sync>,
    get_label: Box<dyn Fn(&mut Resources) -> String + Send + Sync>,
}

impl ToggleButton {
    pub fn new<A, G>(action: A, get_label: G) -> ToggleButton
    where
        A: FnMut(&mut Resources) -> () + 'static + Send + Sync,
        G: Fn(&mut Resources) -> String + 'static + Send + Sync,
    {
        ToggleButton {
            action: Box::new(action),
            get_label: Box::new(get_label),
        }
    }
}

fn toggle_button_system(world: &mut World, res: &mut Resources) {
    let mut query = world.query_mut::<(
        &mut ToggleButton,
        &Children,
        Mutated<Interaction>,
        &mut Handle<ColorMaterial>,
    )>();
    for (mut toggle_button, _children, interaction, mut material) in &mut query.iter() {
        match *interaction {
            Interaction::Clicked => {
                {
                    let action = &mut *toggle_button.action;
                    action(res);
                }
                let button_materials = res.get::<ButtonMaterials>().unwrap();
                *material = button_materials.pressed;
            }
            Interaction::Hovered => {
                let button_materials = res.get::<ButtonMaterials>().unwrap();
                *material = button_materials.hovered;
            }
            Interaction::None => {
                let button_materials = res.get::<ButtonMaterials>().unwrap();
                *material = button_materials.normal;
            }
        }
    }
}

fn toggle_button_text_system(world: &mut World, res: &mut Resources) {
    let mut set_texts = Vec::new();
    let mut query = world.query::<(&ToggleButton, &Children)>();

    for (toggle_button, children) in &mut query.iter() {
        let get_label = &*toggle_button.get_label;
        set_texts.push((children[0], get_label(res)));
    }

    // cannot mutate Text while iterating over query
    for (ent, s) in set_texts.drain(..) {
        let mut text = world.get_mut::<Text>(ent).unwrap();
        text.value = s;
    }
}
