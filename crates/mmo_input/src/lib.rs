mod actions;
mod bindings;
mod params;
mod state;
pub mod systems;
#[cfg(feature = "tnua")]
pub mod tnua;

pub use bindings::{MmoBindings, UserBinding};
pub use params::MmoMovementParams;
pub use state::{InputSource, MmoMovementMask, MmoMovementState};

use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;

#[derive(Component)]
pub struct MmoMovementContext;

pub struct MmoMovementPlugin {
    bindings: MmoBindings,
}

pub struct MmoMovementPluginBuilder {
    bindings: Option<MmoBindings>,
}

impl MmoMovementPlugin {
    pub fn builder() -> MmoMovementPluginBuilder {
        MmoMovementPluginBuilder { bindings: None }
    }
}

impl MmoMovementPluginBuilder {
    pub fn bindings(mut self, bindings: MmoBindings) -> Self {
        self.bindings = Some(bindings);
        self
    }

    pub fn build(self) -> MmoMovementPlugin {
        MmoMovementPlugin {
            bindings: self.bindings.unwrap_or_default(),
        }
    }
}

impl Plugin for MmoMovementPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<EnhancedInputPlugin>() {
            app.add_plugins(EnhancedInputPlugin);
        }
        app.add_input_context::<MmoMovementContext>();
    }
}
