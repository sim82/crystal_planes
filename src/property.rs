use bevy::prelude::*;
use std::collections::{HashMap, VecDeque};

#[derive(Clone)]
pub enum PropertyValue {
    Bool(bool),
    String(String),
}

pub enum PropertyTransition {
    New(PropertyValue),
    Delete(PropertyValue),
    Change(PropertyValue),
}
pub struct PropertyUpdate {
    pub name: String,
    pub transition: PropertyTransition,
}

pub struct PropertyRegistry {
    properties: HashMap<String, PropertyValue>,
    updates: VecDeque<PropertyUpdate>,
}

impl Default for PropertyRegistry {
    fn default() -> Self {
        PropertyRegistry::new()
    }
}

impl Drop for PropertyRegistry {
    fn drop(&mut self) {
        println!("drop property registry");
    }
}

impl PropertyRegistry {
    pub fn new() -> Self {
        println!("property registry");
        let mut properties = HashMap::new();
        properties.insert("rotator_system.enabled".into(), PropertyValue::Bool(false));
        PropertyRegistry {
            properties,
            updates: VecDeque::new(),
        }
    }
    pub fn insert(&mut self, name: &str, new_value: PropertyValue) {
        match self.properties.entry(name.to_string()) {
            std::collections::hash_map::Entry::Occupied(mut e) => {
                self.updates.push_back(PropertyUpdate {
                    name: name.to_string(),
                    transition: PropertyTransition::Change(e.get().clone()),
                });
                *e.get_mut() = new_value;
            }
            std::collections::hash_map::Entry::Vacant(e) => {
                self.updates.push_back(PropertyUpdate {
                    name: name.to_string(),
                    transition: PropertyTransition::New(new_value.clone()),
                });
                e.insert(new_value);
            }
        }
    }
    pub fn insert_bool(&mut self, name: &str, v: bool) {
        self.insert(name, PropertyValue::Bool(v));
    }

    pub fn get(&self, name: &str) -> Option<&PropertyValue> {
        self.properties.get(name)
    }
    pub fn get_bool(&self, name: &str) -> Option<&bool> {
        if let Some(p) = self.properties.get(name) {
            match p {
                PropertyValue::Bool(v) => Some(v),
                _ => None,
            }
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut PropertyValue> {
        let p = self.properties.get_mut(name);

        match p {
            Some(p) => {
                self.updates.push_back(PropertyUpdate {
                    name: name.to_string(),
                    transition: PropertyTransition::Change(p.clone()),
                });
                Some(p)
            }
            None => None,
        }
    }

    pub fn get_bool_mut(&mut self, name: &str) -> Option<&mut bool> {
        if let Some(p) = self.properties.get_mut(name) {
            match p {
                PropertyValue::Bool(v) => Some(v),
                _ => None,
            }
        } else {
            None
        }
    }
    pub fn drain_updates(&mut self) -> VecDeque<PropertyUpdate> {
        let mut ret = VecDeque::new();
        std::mem::swap(&mut self.updates, &mut ret);
        ret
    }
}

#[derive(Default)]
pub struct PropertyPlugin;

impl Plugin for PropertyPlugin {
    fn build(&self, app: &mut AppBuilder) {
        println!("property plugin");
        app.init_resource::<PropertyRegistry>();
    }
}
