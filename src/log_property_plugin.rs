use bevy::prelude::*;

use std::time::Duration;

use crate::property::{PropertyName, PropertyValue};

pub struct LogPropertyPlugin {
    pub wait_duration: Duration,
}

impl Default for LogPropertyPlugin {
    fn default() -> Self {
        LogPropertyPlugin {
            wait_duration: Duration::from_secs(1),
        }
    }
}

struct LogPropertyState {
    timer: Timer,
}

impl Plugin for LogPropertyPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.insert_resource(LogPropertyState {
            timer: Timer::new(self.wait_duration, true),
        });

        app.add_system_to_stage(CoreStage::PostUpdate, Self::log_property_system.system());
    }
}

impl LogPropertyPlugin {
    fn log_property_system(
        mut state: ResMut<LogPropertyState>,
        time: Res<Time>,
        query: Query<(Entity, &PropertyName, &PropertyValue)>,
    ) {
        if state.timer.tick(time.delta()).finished() {
            info!("=== properties =======");
            for (ent, name, value) in query.iter() {
                info!("property: {} {:?} (ent {:?})", name.0, value, ent);
            }
        }
    }
}
