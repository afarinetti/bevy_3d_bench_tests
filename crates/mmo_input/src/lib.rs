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
        app.add_input_context::<MmoMovementContext>()
            .insert_resource(self.bindings.clone())
            .register_type::<MmoMovementState>()
            .register_type::<MmoMovementMask>()
            .register_type::<MmoMovementParams>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::*;

    #[test]
    fn plugin_builds_without_panic() {
        App::new()
            .add_plugins(MmoMovementPlugin::builder().build());
    }

    #[test]
    fn plugin_with_custom_bindings() {
        let mut custom = MmoBindings::default();
        custom.mouse_sensitivity = 0.005;
        App::new()
            .add_plugins(MmoMovementPlugin::builder().bindings(custom).build());
    }
}
