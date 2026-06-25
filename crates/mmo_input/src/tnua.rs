// Compiled only when feature = "tnua"
use bevy::prelude::*;
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
        // Determine camera yaw: prefer MmoCameraOrientation, fall back to entity facing.
        let camera_yaw = cam_orientation.map(|o| o.yaw).unwrap_or_else(|| {
            // Extract yaw from the entity's current Transform.
            // EulerRot::YXZ gives (yaw, pitch, roll) in Bevy's right-handed Y-up space.
            let (yaw, _pitch, _roll) = transform.rotation.to_euler(EulerRot::YXZ);
            yaw
        });

        // Rotate camera-relative move_intent (Vec2: x=strafe, y=forward) into world-space Vec3.
        // camera_yaw = 0 → +Z forward; positive yaw → clockwise when viewed from above.
        let sin_y = camera_yaw.sin();
        let cos_y = camera_yaw.cos();
        let local = state.move_intent; // x = strafe right, y = forward
        let world_dir = Vec3::new(
            local.x * cos_y + local.y * sin_y, // world X (right)
            0.0,
            -local.x * sin_y + local.y * cos_y, // world Z (forward)
        );

        let effective_speed = params.walk_speed * params.speed_multiplier;
        let desired_motion = world_dir * effective_speed;

        // Face movement direction when moving; keep current facing when still.
        let desired_forward = Dir3::new(world_dir).ok();

        // Set walk basis every frame — Tnua requires it to be driven each frame.
        controller.basis = TnuaBuiltinWalk {
            desired_motion,
            desired_forward,
        };
    }
}
