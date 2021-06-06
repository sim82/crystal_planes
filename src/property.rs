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
pub enum PropertyUpdateEvent {
    Create(String),
    Set(String, PropertyValue),
}

impl PropertyUpdateEvent {
    pub fn new(name: String, value: PropertyValue) -> Self {
        PropertyUpdateEvent::Set(name, value)
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
pub struct PropertyEntityRegistry {
    pub(crate) name_cache: HashMap<String, Entity>,
    pub(crate) value_cache: HashMap<String, PropertyValue>,
    pending_create: Mutex<HashSet<String>>,
}

impl PropertyEntityRegistry {
    pub fn get(&self, name: &str) -> Option<Entity> {
        match self.name_cache.get(name) {
            None => {
                // no mapping exists: trigger creation
                let mut pending_create = self.pending_create.lock().unwrap();
                pending_create.insert(name.to_string());
                None
            }
            Some(ent) => Some(*ent),
        }
    }
    pub fn get_value(&self, name: &str) -> Option<&PropertyValue> {
        self.value_cache.get(name)
    }
    pub fn get_value_bool(&self, name: &str) -> Option<bool> {
        match self.get_value(name) {
            Some(&PropertyValue::Bool(v)) => Some(v),
            _ => None,
        }
    }
}
fn create_pending(
    mut events: EventWriter<PropertyUpdateEvent>,
    mut propent_registry: ResMut<PropertyEntityRegistry>,
) {
    let pending_create = propent_registry.pending_create.get_mut().unwrap();
    if !pending_create.is_empty() {
        // std::mem::take is necessary so we have exclusive mut access inside the loop (pending_create is always completely consumed)
        for pending in std::mem::take(pending_create).drain() {
            println!("send create event for pending propent: {}", pending);
            events.send(PropertyUpdateEvent::Create(pending))
        }
    }
}
fn update_event_listener(
    mut commands: Commands,
    mut property_entity_registry: ResMut<PropertyEntityRegistry>,
    mut events: EventReader<PropertyUpdateEvent>,
    mut query: Query<(Entity, &PropertyName, &mut PropertyValue)>,
    mut query2: Query<(Entity, &PropertyName, &mut PropertyAccess)>,
) {
    let mut updates = HashMap::new();
    for event in events.iter() {
        match event {
            PropertyUpdateEvent::Create(name) => {
                if !property_entity_registry.name_cache.contains_key(name) {
                    commands
                        .spawn()
                        .insert(PropertyName(name.clone()))
                        .insert(PropertyValue::None);
                } else {
                    println!("ignoring redundant create event");
                }
            }
            PropertyUpdateEvent::Set(name, value) => {
                if !property_entity_registry.name_cache.contains_key(name) {
                    commands
                        .spawn()
                        .insert(PropertyName(name.clone()))
                        .insert(value.clone());
                } else {
                    property_entity_registry
                        .value_cache
                        .insert(name.clone(), value.clone());
                    updates.insert(name.clone(), value);
                }
            }
        }
    }
    for (ent, name, mut value) in query.iter_mut() {
        if let Some(new_value) = updates.get(&name.0) {
            println!("propagate update to prop {:?}", ent);
            *value = (*new_value).clone();
            property_entity_registry
                .value_cache
                .insert(name.0.clone(), (*new_value).clone());
        }
    }
    for (ent, name, mut access) in query2.iter_mut() {
        if let Some(new_value) = updates.get(&name.0) {
            println!("propagate update to access {:?}", ent);
            access.cache = (**new_value).clone();
        }
    }
}

fn detect_change(
    mut events: EventWriter<PropertyUpdateEvent>,
    mut propent_registry: ResMut<PropertyEntityRegistry>,
    query: Query<&PropertyValue>,
    query_changed: Query<(Entity, &PropertyName, &PropertyValue), Changed<PropertyName>>,
    mut query_access: Query<(Entity, &PropertyName, &mut PropertyAccess), Changed<PropertyName>>,
) {
    for (ent, name, value) in query_changed.iter() {
        println!("new: {:?} {:?} {:?}", ent, name, value);
        propent_registry.name_cache.insert(name.0.clone(), ent);
        propent_registry
            .value_cache
            .insert(name.0.clone(), value.clone());
    }

    for (ent, name, mut access) in query_access.iter_mut() {
        if !propent_registry.name_cache.contains_key(&name.0) {
            println!("new access for nonexisting. send create event.");
            events.send(PropertyUpdateEvent::Create(name.0.clone()))
        } else {
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
}

#[derive(Default)]
pub struct PropertyPlugin;

impl Plugin for PropertyPlugin {
    fn build(&self, app: &mut AppBuilder) {
        println!("propent plugin");
        app.init_resource::<PropertyEntityRegistry>()
            .add_system(
                create_pending
                    .system()
                    .chain(update_event_listener.system()),
            )
            .add_system(detect_change.system())
            .add_event::<PropertyUpdateEvent>();
    }
}
