//! # mmo_input
//!
//! WoW-style (keyboard/mouse) and FFXIV-style (gamepad) movement controls for Bevy.
//!
//! ## Quick start
//!
//! ```rust,no_run
//! use bevy::prelude::*;
//! use mmo_input::{MmoMovementPlugin, MmoMovementContext, MmoMovementState,
//!                  MmoMovementMask, MmoMovementParams};
//!
//! fn main() {
//!     App::new()
//!         .add_plugins(MmoMovementPlugin::builder().build())
//!         .add_systems(Startup, spawn_player)
//!         .run();
//! }
//!
//! fn spawn_player(mut commands: Commands) {
//!     commands.spawn((
//!         MmoMovementContext,
//!         MmoMovementState::default(),
//!         MmoMovementMask::default(),
//!         MmoMovementParams::default(),
//!     ));
//! }
//! ```
//!
//! ## Tnua integration (feature = "tnua")
//!
//! ```ignore
//! // 1. Add your TnuaScheme (with Basis = TnuaBuiltinWalk) and TnuaController to the player entity.
//! // 2. Add drive_walk_basis — instantiate with your control scheme type:
//! app.add_systems(Update, mmo_input::tnua::drive_walk_basis::<MyActions>);
//!
//! // 3. Feed jump / dash / other Tnua actions yourself:
//! fn my_jump_system(
//!     state: Query<&MmoMovementState>,
//!     mut controller: Query<&mut TnuaController<MyActions>>,
//! ) {
//!     let state = state.single();
//!     if state.jump {
//!         controller.single_mut().action(MyActions::Jump(Default::default()));
//!     }
//! }
//! ```

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
#[cfg(feature = "tnua")]
pub use tnua::MmoCameraOrientation;

use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct MmoMovementContext;

pub struct MmoMovementPlugin {
    bindings: MmoBindings,
}

/// Builder for [`MmoMovementPlugin`].
///
/// # Why there is no `enable_tnua()` method
///
/// The tnua feature exposes a generic system `drive_walk_basis<S>` that the game
/// wires up with its own TnuaController scheme type. A non-generic builder cannot
/// capture `S`, so tnua integration is not a builder option.
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
            .register_type::<MmoMovementContext>()
            .register_type::<MmoMovementState>()
            .register_type::<MmoMovementMask>()
            .register_type::<MmoMovementParams>()
            .register_type::<MmoBindings>()
            .register_type::<InputSource>()
            .register_type::<UserBinding>()
            .add_systems(Update, (
                systems::bindings::rebuild_bindings,
                systems::state::update_state,
            ).chain());

        #[cfg(feature = "tnua")]
        app.register_type::<tnua::MmoCameraOrientation>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
