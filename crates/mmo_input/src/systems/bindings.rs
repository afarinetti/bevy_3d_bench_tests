use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;

use crate::{
    MmoMovementContext,
    actions::*,
    bindings::{MmoBindings, UserBinding},
};

/// Despawns all action+binding entities related to every `MmoMovementContext`
/// entity, then respawns them from the current `MmoBindings` resource.
///
/// Runs each `Update` frame, but short-circuits unless `MmoBindings` was
/// mutated (or just inserted on the first frame).
///
/// # Relationship model
///
/// bevy_enhanced_input 0.26 does **not** use parent-child (`Children`) for
/// actions or bindings.  Instead it uses Bevy's ECS relationship system:
/// - Action entities carry `ActionOf<C>` (pointing to the context entity).
/// - The context entity accumulates them in `Actions<C>`.
/// - Binding entities carry `BindingOf` (pointing to their action entity).
/// - The action entity accumulates them in `Bindings`.
/// - `Bindings` is `linked_spawn`, so despawning an action entity cascades
///   to despawn its binding entities automatically.
pub fn rebuild_bindings(
    mut commands: Commands,
    bindings: Res<MmoBindings>,
    context_query: Query<Entity, With<MmoMovementContext>>,
) {
    if !bindings.is_changed() {
        return;
    }

    for ctx_entity in context_query.iter() {
        // Despawn existing action entities; their Bindings cascade-despawn.
        commands
            .entity(ctx_entity)
            .despawn_related::<Actions<MmoMovementContext>>();

        // Respawn all action entities with their bindings.
        spawn_actions(&mut commands, ctx_entity, &bindings);
    }
}

fn spawn_actions(commands: &mut Commands, ctx: Entity, b: &MmoBindings) {
    // Combine jump + gamepad jump into a single Jump action.
    let jump_bindings: Vec<UserBinding> = b
        .jump
        .iter()
        .chain(b.gamepad_jump.iter())
        .cloned()
        .collect();

    // Combine keyboard sprint + gamepad sprint into a single Sprint action.
    let sprint_bindings: Vec<UserBinding> = b
        .sprint
        .iter()
        .chain(b.gamepad_sprint.iter())
        .cloned()
        .collect();

    commands
        .entity(ctx)
        .with_related_entities::<ActionOf<MmoMovementContext>>(|spawner| {
            spawn_bool_action::<MoveForward>(spawner, &b.move_forward);
            spawn_bool_action::<MoveBackward>(spawner, &b.move_backward);
            spawn_bool_action::<TurnLeft>(spawner, &b.turn_left);
            spawn_bool_action::<TurnRight>(spawner, &b.turn_right);
            spawn_bool_action::<StrafeLeft>(spawner, &b.strafe_left);
            spawn_bool_action::<StrafeRight>(spawner, &b.strafe_right);
            spawn_bool_action::<Jump>(spawner, &jump_bindings);
            spawn_bool_action::<MouseLook>(spawner, &b.mouse_look);
            spawn_bool_action::<LeftMouse>(spawner, &b.left_mouse);
            spawn_bool_action::<AutoRun>(spawner, &b.auto_run);
            spawn_bool_action::<Sprint>(spawner, &sprint_bindings);

            // MouseDelta — always bound to mouse motion (not rebindable).
            let mut mouse_delta = spawner.spawn(Action::<MouseDelta>::new());
            mouse_delta.insert(ActionSettings {
                consume_input: false,
                ..default()
            });
            mouse_delta.with_related_entities::<BindingOf>(|bs| {
                bs.spawn(Binding::mouse_motion());
            });

            // GamepadMove — left stick X + Y axes (not rebindable).
            let mut gp_move = spawner.spawn(Action::<GamepadMove>::new());
            gp_move.insert(ActionSettings {
                consume_input: false,
                ..default()
            });
            gp_move.with_related_entities::<BindingOf>(|bs| {
                bs.spawn(Binding::GamepadAxis(GamepadAxis::LeftStickX));
                bs.spawn(Binding::GamepadAxis(GamepadAxis::LeftStickY));
            });

            // GamepadCamera — right stick X + Y axes (not rebindable).
            let mut gp_cam = spawner.spawn(Action::<GamepadCamera>::new());
            gp_cam.insert(ActionSettings {
                consume_input: false,
                ..default()
            });
            gp_cam.with_related_entities::<BindingOf>(|bs| {
                bs.spawn(Binding::GamepadAxis(GamepadAxis::RightStickX));
                bs.spawn(Binding::GamepadAxis(GamepadAxis::RightStickY));
            });
        });
}

/// Spawns one `Action<A>` entity (related to the context via the spawner)
/// and then spawns one binding entity per `UserBinding`.
fn spawn_bool_action<A: InputAction<Output = bool> + 'static>(
    spawner: &mut ActionSpawnerCommands<'_, MmoMovementContext>,
    user_bindings: &[UserBinding],
) {
    let mut action_cmd = spawner.spawn(Action::<A>::new());
    action_cmd.insert(ActionSettings {
        consume_input: false,
        ..default()
    });
    action_cmd.with_related_entities::<BindingOf>(|bs| {
        for ub in user_bindings {
            bs.spawn(ub.to_bevy_binding());
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MmoBindings, MmoMovementContext, MmoMovementPlugin};

    fn make_app() -> App {
        let mut app = App::new();
        // MinimalPlugins provides Time resources needed by bevy_enhanced_input's
        // update system (ContextTime: Res<Time<Real>>, Res<Time<Virtual>>).
        app.add_plugins((bevy::app::ScheduleRunnerPlugin::default(), bevy::time::TimePlugin));
        app.add_plugins(MmoMovementPlugin::builder().build());
        // finish() initializes ContextInstances<S> so that the registered
        // observers (register, unregister, deactivate, reset_action) don't panic.
        app.finish();
        app
    }

    /// After one `app.update()` on a frame where the plugin just inserted
    /// `MmoBindings`, the `rebuild_bindings` system should fire (resource
    /// `is_changed()` == true) and populate `Actions<MmoMovementContext>` on
    /// the context entity.
    #[test]
    fn spawning_context_entity_creates_action_children() {
        let mut app = make_app();
        let ctx = app
            .world_mut()
            .spawn((
                MmoMovementContext,
                crate::MmoMovementState::default(),
                crate::MmoMovementMask::default(),
            ))
            .id();

        // Run one frame: rebuild_bindings fires, commands are applied.
        app.update();

        let actions = app
            .world()
            .entity(ctx)
            .get::<Actions<MmoMovementContext>>();
        assert!(
            actions.is_some(),
            "context entity should have Actions<MmoMovementContext> after first update"
        );
        assert!(
            !actions.unwrap().is_empty(),
            "context entity should have at least one action entity"
        );
    }

    /// Mutating `MmoBindings` should cause `rebuild_bindings` to despawn the
    /// old action entities and respawn the same number of new ones.
    #[test]
    fn changing_bindings_resource_rebuilds_action_children() {
        let mut app = make_app();
        let ctx = app
            .world_mut()
            .spawn((
                MmoMovementContext,
                crate::MmoMovementState::default(),
                crate::MmoMovementMask::default(),
            ))
            .id();
        app.update();

        let action_count_before = app
            .world()
            .entity(ctx)
            .get::<Actions<MmoMovementContext>>()
            .map(|a| a.len())
            .unwrap_or(0);

        // Mutate bindings (changing sensitivity doesn't affect action count).
        app.world_mut().resource_mut::<MmoBindings>().mouse_sensitivity = 0.999;
        app.update();

        let action_count_after = app
            .world()
            .entity(ctx)
            .get::<Actions<MmoMovementContext>>()
            .map(|a| a.len())
            .unwrap_or(0);

        assert!(
            action_count_before > 0,
            "expected actions before rebuild, got 0"
        );
        assert_eq!(
            action_count_before, action_count_after,
            "action count must be the same after rebuild"
        );
    }
}
