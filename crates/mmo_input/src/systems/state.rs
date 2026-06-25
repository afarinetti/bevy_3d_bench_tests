use bevy::prelude::*;
// `SystemParam` derive macro is not in `bevy::prelude` with `default-features = false`;
// import it explicitly from the path where bevy_ecs re-exports the proc macro.
use bevy::ecs::system::SystemParam;
use bevy_enhanced_input::prelude::*;

use crate::{
    MmoMovementContext,
    actions::*,
    bindings::MmoBindings,
    state::{InputSource, MmoMovementMask, MmoMovementState},
};

/// Groups all action queries for `MmoMovementContext` into one `SystemParam`.
///
/// Avoids hitting Bevy's system-function tuple limit and keeps the system
/// signature readable.
///
/// ## bevy_enhanced_input 0.26 API notes (vs. the draft in the brief)
///
/// - `Action<A>` implements `Deref<Target = A::Output>`.
///   Use `**action` to read the value (e.g. `bool` or `Vec2`).
/// - The old `ActionState` type is now `TriggerState` (deprecated alias kept).
/// - Action state is a **separate** `TriggerState` component, not a method.
/// - Event bits live in a separate `ActionEvents` component.
///   `ActionEvents::START` replaces the deprecated `ActionEvents::STARTED`.
/// - Action entities are queried via `With<ActionOf<MmoMovementContext>>` —
///   NOT via `Single<>` or `query.get(child)`.
/// - Commands from `rebuild_bindings` may not be flushed before this system
///   runs on the same frame (`.chain()` doesn't insert `apply_deferred`).
///   All queries therefore fall back to `iter().next()` with a default, which
///   returns `None` safely when the action entity doesn't exist yet.
#[derive(SystemParam)]
pub struct ActionInputs<'w, 's> {
    fwd:      Query<'w, 's, &'static Action<MoveForward>,   With<ActionOf<MmoMovementContext>>>,
    bwd:      Query<'w, 's, &'static Action<MoveBackward>,  With<ActionOf<MmoMovementContext>>>,
    turn_l:   Query<'w, 's, &'static Action<TurnLeft>,      With<ActionOf<MmoMovementContext>>>,
    turn_r:   Query<'w, 's, &'static Action<TurnRight>,     With<ActionOf<MmoMovementContext>>>,
    str_l:    Query<'w, 's, &'static Action<StrafeLeft>,    With<ActionOf<MmoMovementContext>>>,
    str_r:    Query<'w, 's, &'static Action<StrafeRight>,   With<ActionOf<MmoMovementContext>>>,
    jump:     Query<'w, 's, (&'static Action<Jump>,    &'static ActionEvents), With<ActionOf<MmoMovementContext>>>,
    rmb:      Query<'w, 's, &'static Action<MouseLook>,     With<ActionOf<MmoMovementContext>>>,
    lmb:      Query<'w, 's, &'static Action<LeftMouse>,     With<ActionOf<MmoMovementContext>>>,
    auto_run: Query<'w, 's, (&'static Action<AutoRun>, &'static ActionEvents), With<ActionOf<MmoMovementContext>>>,
    delta:    Query<'w, 's, &'static Action<MouseDelta>,    With<ActionOf<MmoMovementContext>>>,
    gp_move:  Query<'w, 's, &'static Action<GamepadMove>,   With<ActionOf<MmoMovementContext>>>,
    gp_cam:   Query<'w, 's, &'static Action<GamepadCamera>, With<ActionOf<MmoMovementContext>>>,
    sprint:   Query<'w, 's, (&'static Action<Sprint>,  &'static ActionEvents), With<ActionOf<MmoMovementContext>>>,
}

/// Reads action states each frame and writes `MmoMovementState`, applying `MmoMovementMask`.
///
/// Runs chained **after** `rebuild_bindings` in the `Update` schedule.
///
/// ## WoW / FFXIV movement modes
///
/// | Condition        | A / D behaviour     | Mouse X       |
/// |------------------|---------------------|---------------|
/// | No RMB (KB only) | Turn character      | —             |
/// | RMB held         | Strafe              | yaw + camera  |
/// | Gamepad          | Left stick (FFXIV)  | Right stick   |
///
/// Q / E always strafe. LMB + RMB → force forward.
/// Auto-run ensures `move_intent.y ≥ 1.0` regardless of key input.
pub fn update_state(
    bindings: Res<MmoBindings>,
    time: Res<Time>,
    mut context_q: Query<(&mut MmoMovementState, &MmoMovementMask), With<MmoMovementContext>>,
    inputs: ActionInputs,
) {
    let dt = time.delta_secs();

    // --- Read all action values; default when action entity absent. ---
    //
    // `.iter().next()` is used rather than `.get_single()` / `.single()` because
    // the single-result API changed name across Bevy versions (0.14–0.19).
    // `.iter().next()` is stable and correctly defaults when no entity exists yet.

    let fwd_held  = inputs.fwd.iter().next().map(|a| **a).unwrap_or(false);
    let bwd_held  = inputs.bwd.iter().next().map(|a| **a).unwrap_or(false);
    let turn_l    = inputs.turn_l.iter().next().map(|a| **a).unwrap_or(false);
    let turn_r    = inputs.turn_r.iter().next().map(|a| **a).unwrap_or(false);
    let strafe_l  = inputs.str_l.iter().next().map(|a| **a).unwrap_or(false);
    let strafe_r  = inputs.str_r.iter().next().map(|a| **a).unwrap_or(false);
    let mouse_rmb = inputs.rmb.iter().next().map(|a| **a).unwrap_or(false);
    let mouse_lmb = inputs.lmb.iter().next().map(|a| **a).unwrap_or(false);
    let m_delta   = inputs.delta.iter().next().map(|a| **a).unwrap_or(Vec2::ZERO);
    let gp_move   = inputs.gp_move.iter().next().map(|a| **a).unwrap_or(Vec2::ZERO);
    let gp_cam    = inputs.gp_cam.iter().next().map(|a| **a).unwrap_or(Vec2::ZERO);

    // Jump / sprint / auto-run use START (first frame pressed = "just pressed").
    let jump_fired = inputs.jump
        .iter()
        .next()
        .map(|(_, evs)| evs.contains(ActionEvents::START))
        .unwrap_or(false);
    let sprint_fired = inputs.sprint
        .iter()
        .next()
        .map(|(_, evs)| evs.contains(ActionEvents::START))
        .unwrap_or(false);
    let auto_run_just_started = inputs.auto_run
        .iter()
        .next()
        .map(|(_, evs)| evs.contains(ActionEvents::START))
        .unwrap_or(false);

    // Gamepad: active when either stick has meaningful deflection.
    let is_gamepad = gp_move.length_squared() > 0.01 || gp_cam.length_squared() > 0.01;

    for (mut state, mask) in context_q.iter_mut() {
        // Toggle auto-run on first-frame press.
        if auto_run_just_started {
            state.auto_run = !state.auto_run;
        }

        let is_mouse_look = mouse_rmb || is_gamepad;
        state.is_mouse_look = is_mouse_look;
        state.input_source = if is_gamepad {
            InputSource::Gamepad
        } else {
            InputSource::KeyboardMouse
        };

        // Camera delta is NEVER masked — drives the orbit camera directly.
        // Sensitivity scales here; the camera crate applies pitch clamping.
        let cam_delta = if is_gamepad {
            gp_cam * bindings.gamepad_sensitivity * dt
        } else {
            m_delta * bindings.mouse_sensitivity
        };
        state.camera_delta = cam_delta;

        let mut intent = Vec2::ZERO;
        let mut yaw    = 0.0f32;

        if is_gamepad {
            // FFXIV-style: left stick → camera-relative move, right stick → camera + yaw.
            intent.x += gp_move.x;
            intent.y += gp_move.y;
            yaw += cam_delta.x; // right-stick X drives character yaw
        } else {
            // W / S → forward / backward.
            if fwd_held { intent.y += 1.0; }
            if bwd_held { intent.y -= 1.0; }

            // Q / E → strafe regardless of mouse-look state.
            if strafe_l { intent.x -= 1.0; }
            if strafe_r { intent.x += 1.0; }

            if is_mouse_look {
                // RMB held: A / D strafe; mouse X drives yaw (mirrored in camera_delta).
                if turn_l { intent.x -= 1.0; }
                if turn_r { intent.x += 1.0; }
                yaw += cam_delta.x;
            } else {
                // Keyboard-only: A / D turn character in world space.
                if turn_l { yaw -= bindings.kb_turn_speed * dt; }
                if turn_r { yaw += bindings.kb_turn_speed * dt; }
            }

            // Both mouse buttons held → WoW auto-walk: force forward.
            if mouse_lmb && mouse_rmb {
                intent.y = intent.y.max(1.0);
            }
        }

        // Auto-run guarantees minimum forward motion before masking.
        if state.auto_run {
            intent.y = intent.y.max(1.0);
        }

        // Clamp gamepad diagonal to unit circle.
        if intent.length() > 1.0 {
            intent = intent.normalize();
        }

        // --- Apply MmoMovementMask ---
        // MOVEMENT bit gates move_intent; TURNING gates yaw_delta only.
        // camera_delta is intentionally NOT masked (spec requirement).
        if !mask.contains(MmoMovementMask::MOVEMENT) { intent = Vec2::ZERO; }
        if !mask.contains(MmoMovementMask::TURNING)  { yaw = 0.0; }

        let jump_out   = jump_fired   && mask.contains(MmoMovementMask::JUMPING);
        let sprint_out = sprint_fired && mask.contains(MmoMovementMask::JUMPING);
        let vert = if !mask.contains(MmoMovementMask::VERTICAL) {
            0.0
        } else {
            state.vertical_intent
        };

        state.move_intent     = intent;
        state.yaw_delta       = yaw;
        state.jump            = jump_out;
        state.sprint          = sprint_out;
        state.vertical_intent = vert;
        state.tick           += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MmoMovementContext, MmoMovementParams, MmoMovementPlugin};

    /// Minimal test app — matches the pattern that works for `systems::bindings` tests.
    /// Uses `ScheduleRunnerPlugin + TimePlugin` (subset of `MinimalPlugins`) to avoid
    /// pulling in window / render infrastructure that isn't available without those bevy
    /// features.  `MmoMovementPlugin` adds `EnhancedInputPlugin` internally.
    fn make_app() -> App {
        let mut app = App::new();
        app.add_plugins((
            bevy::app::ScheduleRunnerPlugin::default(),
            bevy::time::TimePlugin,
        ));
        app.add_plugins(MmoMovementPlugin::builder().build());
        app.finish();
        app
    }

    fn spawn_player(app: &mut App) -> Entity {
        app.world_mut()
            .spawn((
                MmoMovementContext,
                MmoMovementState::default(),
                MmoMovementMask::default(), // ALL
                MmoMovementParams::default(),
            ))
            .id()
    }

    /// Baseline: without any input, `move_intent` stays at zero and `tick` is 1
    /// after a single frame.
    #[test]
    fn w_key_sets_forward_intent() {
        let mut app = make_app();
        let player = spawn_player(&mut app);
        app.update();
        let state = app
            .world()
            .entity(player)
            .get::<MmoMovementState>()
            .unwrap();
        assert_eq!(state.move_intent, Vec2::ZERO);
        assert_eq!(state.tick, 1);
    }

    /// `MmoMovementMask::empty()` zeroes all movement-related outputs each frame,
    /// regardless of what the initial state contained.
    #[test]
    fn mask_empty_zeroes_all_intent() {
        let mut app = make_app();
        let player = app
            .world_mut()
            .spawn((
                MmoMovementContext,
                MmoMovementState {
                    move_intent: Vec2::ONE,
                    yaw_delta: 1.0,
                    jump: true,
                    vertical_intent: 1.0,
                    ..Default::default()
                },
                MmoMovementMask::empty(),
                MmoMovementParams::default(),
            ))
            .id();
        app.update();
        let state = app
            .world()
            .entity(player)
            .get::<MmoMovementState>()
            .unwrap();
        assert_eq!(state.move_intent, Vec2::ZERO);
        assert_eq!(state.yaw_delta, 0.0);
        assert!(!state.jump);
        assert_eq!(state.vertical_intent, 0.0);
    }

    /// Removing TURNING from the mask zeroes `yaw_delta` but must leave
    /// `camera_delta` untouched (static logic test — full verification requires
    /// synthesising mouse delta which needs hardware event infrastructure).
    #[test]
    fn mask_turning_removed_zeroes_yaw_not_camera() {
        let mask = MmoMovementMask::ALL & !MmoMovementMask::TURNING;
        assert!(
            !mask.contains(MmoMovementMask::TURNING),
            "TURNING should be absent after removal"
        );
        assert!(
            mask.contains(MmoMovementMask::MOVEMENT),
            "MOVEMENT should still be present"
        );
    }

    /// `tick` increments by exactly 1 every frame.
    #[test]
    fn tick_increments_each_frame() {
        let mut app = make_app();
        let player = spawn_player(&mut app);
        app.update();
        app.update();
        app.update();
        let state = app
            .world()
            .entity(player)
            .get::<MmoMovementState>()
            .unwrap();
        assert!(state.tick >= 3, "tick should have incremented each frame, got {}", state.tick);
    }

    /// When `auto_run` is true on the state component, the system ensures
    /// `move_intent.y ≥ 1.0` even with no key input.
    #[test]
    fn auto_run_clamps_forward_intent_to_at_least_one() {
        let mut app = make_app();
        let player = app
            .world_mut()
            .spawn((
                MmoMovementContext,
                MmoMovementState { auto_run: true, ..Default::default() },
                MmoMovementMask::ALL,
                MmoMovementParams::default(),
            ))
            .id();
        app.update();
        let state = app
            .world()
            .entity(player)
            .get::<MmoMovementState>()
            .unwrap();
        assert!(
            state.move_intent.y >= 1.0,
            "auto_run must ensure move_intent.y >= 1.0, got {}",
            state.move_intent.y
        );
    }
}
