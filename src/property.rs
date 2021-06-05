use bevy::prelude::*;
use std::{collections::HashMap, sync::atomic::AtomicBool};

#[derive(Clone, Debug, PartialEq)]
pub enum PropertyValue {
    None,
    Bool(bool),
    String(String),
}

#[derive(Default)]
pub struct PropertyRegistry {
    properties: HashMap<String, PropertyValue>,
    update_listeners: HashMap<String, Vec<std::sync::Arc<std::sync::atomic::AtomicBool>>>,
}

impl PropertyRegistry {
    pub fn insert(&mut self, name: &str, new_value: PropertyValue) {
        self.properties.insert(name.into(), new_value);
        self.signal(name);
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
        self.signal(name); // FIXME: find some way to call only on success
        let p = self.properties.get_mut(name);
        match p {
            Some(p) => Some(p),
            None => None,
        }
    }

    pub fn get_bool_mut(&mut self, name: &str) -> Option<&mut bool> {
        self.signal(name); // FIXME: find some way to call only on success

        if let Some(p) = self.properties.get_mut(name) {
            match p {
                PropertyValue::Bool(v) => Some(v),
                _ => None,
            }
        } else {
            None
        }
    }

    fn signal(&mut self, name: &str) {
        if let Some(listeners) = self.update_listeners.get_mut(name) {
            // listeners.drain_filter(
            //     |sender| sender.send(()).is
            // );

            for listener in listeners {
                listener.store(true, std::sync::atomic::Ordering::Relaxed)
            }
        }
    }

    pub fn subscribe(&mut self, name: &str) -> std::sync::Arc<std::sync::atomic::AtomicBool> {
        let b = std::sync::Arc::new(AtomicBool::new(true));
        match self.update_listeners.entry(name.to_string()) {
            std::collections::hash_map::Entry::Occupied(mut l) => l.get_mut().push(b.clone()),
            std::collections::hash_map::Entry::Vacant(e) => {
                e.insert(vec![b.clone()]);
            }
        }
        b
    }
}

pub struct PropertyTracker {
    pub name: String,
    current_value: PropertyValue,
    update_signal: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
}

impl PropertyTracker {
    pub fn new() -> Self {
        PropertyTracker {
            name: String::new(),
            current_value: PropertyValue::None,
            update_signal: None,
        }
    }
    pub fn new_subscribed(property_registry: &mut PropertyRegistry, name: &str) -> Self {
        let mut t = Self::new();
        t.subscribe(property_registry, name);
        t
    }
    pub fn subscribe(&mut self, property_registry: &mut PropertyRegistry, name: &str) {
        if self.update_signal.is_some() {
            return;
        }
        self.update_signal = Some(property_registry.subscribe(name));
        self.name = name.to_string();
    }
    pub fn get_changed(&mut self, property_registry: &PropertyRegistry) -> Option<&PropertyValue> {
        if let Some(update_signal) = &self.update_signal {
            if update_signal.load(std::sync::atomic::Ordering::Relaxed) {
                update_signal.store(false, std::sync::atomic::Ordering::Relaxed);
                if let Some(value) = property_registry.get(&self.name) {
                    if *value != self.current_value {
                        self.current_value = value.clone();
                        return Some(&self.current_value);
                    }
                }
            }
        }
        None
    }
}

impl Default for PropertyTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl DetectChanges for PropertyTracker {
    fn is_added(&self) -> bool {
        false
    }

    fn is_changed(&self) -> bool {
        println!("is_changed");
        if let Some(update_signal) = &self.update_signal {
            update_signal.load(std::sync::atomic::Ordering::Relaxed)
        } else {
            false
        }
    }

    fn set_changed(&mut self) {
        if let Some(update_signal) = &mut self.update_signal {
            update_signal.store(true, std::sync::atomic::Ordering::Relaxed);
        }
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
