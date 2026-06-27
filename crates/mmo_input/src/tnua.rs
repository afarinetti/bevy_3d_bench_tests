// Compiled only when feature = "tnua"
use bevy::prelude::*;
use bevy_tnua::builtins::TnuaBuiltinWalkConfig;
use bevy_tnua::prelude::*;

use crate::{MmoMovementContext, MmoMovementParams, MmoMovementState};

/// Optional camera orientation written by the `mmo_camera` crate.
///
/// Attach this component to the player entity alongside [`MmoMovementContext`].
/// If absent, [`drive_walk_basis`] falls back to the entity's own `Transform` forward direction.
#[derive(Component, Reflect, Default, Clone, Copy)]
#[reflect(Component)]
pub struct MmoCameraOrientation {
    /// World-space yaw in radians (0 = +Z forward, positive yaw = clockwise from above).
    pub yaw: f32,
    /// Pitch in radians (clamped by `mmo_camera`).
    pub pitch: f32,
    /// Current zoom distance in world units (informational only).
    pub distance: f32,
}

/// Drive the [`TnuaBuiltinWalk`] basis from [`MmoMovementState`] every frame.
///
/// Add this system to your app when using the `tnua` feature.
/// `S` must be your `TnuaScheme` with `Basis = TnuaBuiltinWalk`.
///
/// ```ignore
/// use mmo_input::tnua::drive_walk_basis;
/// app.add_systems(Update, drive_walk_basis::<MyActions>.after(mmo_input_update_system));
/// ```
///
/// Walk speed and acceleration come from [`MmoMovementParams`] on the entity.
/// Camera-relative direction is read from [`MmoCameraOrientation`] when present.
///
/// # Wiring actions (jump, dash, …)
///
/// This system only sets the basis. Feed Tnua actions yourself:
///
/// ```ignore
/// fn my_jump_system(
///     state: Query<&MmoMovementState>,
///     mut controller: Query<&mut TnuaController<MyActions>>,
/// ) {
///     let state = state.single();
///     if state.jump {
///         controller.single_mut().action(MyActions::Jump(Default::default()));
///     }
/// }
/// ```
pub fn drive_walk_basis<S>(
    mut query: Query<
        (
            &MmoMovementState,
            &MmoMovementParams,
            &mut TnuaController<S>,
            Option<&MmoCameraOrientation>,
            &Transform,
        ),
        With<MmoMovementContext>,
    >,
) where
    S: TnuaScheme<Basis = TnuaBuiltinWalk>,
{
    for (state, params, mut controller, cam_orientation, transform) in query.iter_mut() {
        let local = state.move_intent; // x = strafe right, y = forward

        // Rotate camera-relative move_intent into a world-space horizontal Vec3.
        let world_dir = match cam_orientation {
            Some(o) => {
                // `MmoCameraOrientation.yaw` convention (see the type's docs):
                // yaw = 0 → +Z forward; positive yaw → clockwise viewed from above.
                // NOTE: this assumes the `mmo_camera` crate writes `yaw` in that
                // convention. It is a separate crate and out of scope here.
                let sin_y = o.yaw.sin();
                let cos_y = o.yaw.cos();
                Vec3::new(
                    local.x * cos_y + local.y * sin_y, // world X (right)
                    0.0,
                    -local.x * sin_y + local.y * cos_y, // world Z (forward)
                )
            }
            None => {
                // No camera orientation: fall back to the entity's own facing.
                // Use `transform.forward()` / `right()` directly — Bevy's forward
                // is -Z, so re-deriving a yaw and feeding the "+Z forward" formula
                // above would flip the sign and drive the character backward.
                let forward = transform.forward();
                let right = transform.right();
                let mut dir = right * local.x + forward * local.y;
                dir.y = 0.0; // keep the walk basis horizontal
                dir
            }
        };

        let effective_speed = params.walk_speed * params.speed_multiplier;
        let desired_motion = world_dir * effective_speed;

        // Face movement direction when moving; keep current facing when still.
        let desired_forward = Dir3::new(world_dir).ok();

        // Set walk basis every frame — Tnua requires it to be driven each frame.
        controller.basis = TnuaBuiltinWalk {
            desired_motion,
            desired_forward,
        };

        // Propagate MmoMovementParams into the walk config so Tnua uses the
        // correct float_height, acceleration, and coyote_time.
        //
        // `speed` is 1.0 because `desired_motion` already encodes the full
        // velocity (walk_speed × speed_multiplier × direction); see above.
        //
        // NOTE: In bevy-tnua 0.32, `apply_controller_system` overwrites
        // `basis_config` every physics tick from the `TnuaConfig<S>` asset.
        // If your setup uses `TnuaConfig<S>`, mirror float_height, acceleration,
        // air_acceleration, and coyote_time in your `TnuaSchemeConfig` asset as
        // well. This assignment takes effect in configurations that bypass the
        // asset system (e.g. headless tests or custom controller pipelines).
        controller.basis_config = Some(TnuaBuiltinWalkConfig {
            speed: 1.0,
            float_height: params.float_height,
            acceleration: params.acceleration,
            air_acceleration: params.air_acceleration,
            coyote_time: params.coyote_time,
            ..TnuaBuiltinWalkConfig::default()
        });
    }
}
