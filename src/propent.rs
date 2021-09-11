use bevy::prelude::*;
use std::{
    collections::{HashMap, HashSet},
    sync::Mutex,
};

#[derive(Debug, Clone)]
pub struct PropertyName(pub String);

#[derive(Clone, Debug, PartialEq)]
pub enum PropertyValue {
    None,
    Bool(bool),
    String(String),
}

#[derive(Debug)]
pub struct PropertyUpdateEvent {
    name: String,
    value: PropertyValue,
}

impl PropertyUpdateEvent {
    pub fn new(name: String, value: PropertyValue) -> Self {
        PropertyUpdateEvent { name, value }
    }
}

pub struct PropertyAccess {
    pub cache: PropertyValue,
}

impl Default for PropertyAccess {
    fn default() -> Self {
        PropertyAccess {
            cache: PropertyValue::None,
        }
    }
}

#[derive(Default)]
pub struct PropentRegistry {
    pub(crate) name_cache: HashMap<String, Option<Entity>>,
    pending_create: Mutex<HashSet<String>>,
}
// impl Default for PropentRegistry {
//     fn default() -> Self {
//         PropentRegistry {
//             name_cache:
//         }
//     }
// }

impl PropentRegistry {
    pub fn get(&self, name: &str) -> Option<Entity> {
        match self.name_cache.get(name) {
            None => {
                // no mapping exists: trigger creation
                let mut pending_create = self.pending_create.lock().unwrap();
                pending_create.insert(name.to_string());
                None
            }
            Some(None) => None, // entity already under construction
            Some(Some(ent)) => Some(*ent),
        }
    }
}
fn create_pending(mut commands: Commands, mut propent_registry: ResMut<PropentRegistry>) {
    let pending_create = propent_registry.pending_create.get_mut().unwrap();
    if !pending_create.is_empty() {
        // std::mem::take is necessary so we have exclusive mut access inside the loop (pending_create is always completely consumed)
        for pending in std::mem::take(pending_create).drain() {
            println!("spawn pending propent: {}", pending);
            propent_registry.name_cache.insert(pending.clone(), None); // placeholder, will be filled by detect_change system
            commands
                .spawn()
                .insert(PropertyName(pending))
                .insert(PropertyValue::None);
        }
    }
}
fn detect_change(
    mut propent_registry: ResMut<PropentRegistry>,
    query: Query<&PropertyValue>,
    query_changed: Query<(Entity, &PropertyName, &PropertyValue), Changed<PropertyName>>,
    mut query_access: Query<(Entity, &PropertyName, &mut PropertyAccess), Changed<PropertyName>>,
) {
    for (ent, name, value) in query_changed.iter() {
        println!("new: {:?} {:?} {:?}", ent, name, value);
        propent_registry
            .name_cache
            .insert(name.0.clone(), Some(ent));
    }

    for (ent, name, mut access) in query_access.iter_mut() {
        println!("new access. initial propagate: {:?} {:?}", ent, name);
        let value = query
            .get(
                propent_registry
                    .get(&name.0)
                    .expect("failed to get ent for property"),
            )
            .expect("missing property value for access");

        access.cache = value.clone();
    }
}

fn update_event_listener(
    mut events: EventReader<PropertyUpdateEvent>,
    mut query: Query<(Entity, &PropertyName, &mut PropertyValue)>,
    mut query2: Query<(Entity, &PropertyName, &mut PropertyAccess)>,
) {
    let mut updates = HashMap::new();
    for event in events.iter() {
        println!("update: {:?}", event);
        updates.insert(&event.name, &event.value);
    }
    for (ent, name, mut value) in query.iter_mut() {
        if let Some(new_value) = updates.get(&name.0) {
            println!("propagate update to prop {:?}", ent);
            *value = (**new_value).clone();
        }
    }
    for (ent, name, mut access) in query2.iter_mut() {
        if let Some(new_value) = updates.get(&name.0) {
            println!("propagate update to access {:?}", ent);
            access.cache = (**new_value).clone();
        }
    }
}

#[derive(Default)]
pub struct PropentPlugin;

impl Plugin for PropentPlugin {
    fn build(&self, app: &mut App) {
        println!("propent plugin");
        app.init_resource::<PropentRegistry>()
            .add_system(create_pending.system())
            .add_system(detect_change.system())
            .add_system(update_event_listener.system())
            .add_event::<PropertyUpdateEvent>();
    }
}
