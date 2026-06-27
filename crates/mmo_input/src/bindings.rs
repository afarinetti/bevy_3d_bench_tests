use bevy::input::{gamepad::GamepadButton, keyboard::KeyCode, mouse::MouseButton};
use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;
use serde::{Deserialize, Serialize};

/// Serde-friendly wrapper over a rebindable physical input.
///
/// Gamepad axes (left/right stick) are hardcoded and not represented here.
///
/// # Scroll wheel note
/// `Binding::MouseWheel` in bevy_enhanced_input 0.26 captures the full scroll
/// axis (`Axis2D`). There is no separate "scroll up" / "scroll down" mouse
/// button variant in Bevy 0.19 (`MouseButton::Other(u16)` is for side
/// buttons, not the scroll wheel). Both `zoom_in` and `zoom_out` therefore
/// store `UserBinding::MouseWheel`; the consumer crate (`mmo_camera`) must
/// distinguish direction via a `Negate` modifier on the zoom-out action.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Reflect)]
pub enum UserBinding {
    Key(KeyCode),
    Mouse(MouseButton),
    Gamepad(GamepadButton),
    /// Maps to `Binding::MouseWheel`. Direction is controlled by modifiers in
    /// the consuming crate.
    MouseWheel,
}

impl UserBinding {
    pub fn to_bevy_binding(&self) -> Binding {
        match self {
            UserBinding::Key(k) => Binding::from(*k),
            UserBinding::Mouse(m) => Binding::from(*m),
            UserBinding::Gamepad(b) => Binding::from(*b),
            UserBinding::MouseWheel => Binding::mouse_wheel(),
        }
    }
}

/// Per-player rebindable input map with WoW/FFXIV-style defaults.
///
/// Stored as a `Resource`; insert via `MmoMovementPlugin::builder()`.
#[derive(Resource, Reflect, Serialize, Deserialize, Clone)]
pub struct MmoBindings {
    // ── Keyboard / mouse (WoW-style defaults) ───────────────────────────────
    pub move_forward: Vec<UserBinding>,
    pub move_backward: Vec<UserBinding>,
    pub turn_left: Vec<UserBinding>,
    pub turn_right: Vec<UserBinding>,
    pub strafe_left: Vec<UserBinding>,
    pub strafe_right: Vec<UserBinding>,
    pub jump: Vec<UserBinding>,
    pub mouse_look: Vec<UserBinding>,
    pub left_mouse: Vec<UserBinding>,
    pub auto_run: Vec<UserBinding>,
    /// Empty by default (WoW has no keyboard sprint). Assign e.g. `Shift` here.
    pub sprint: Vec<UserBinding>,

    // ── Gamepad (FFXIV-style defaults) ──────────────────────────────────────
    pub gamepad_jump: Vec<UserBinding>,
    pub gamepad_sprint: Vec<UserBinding>,

    // ── Camera zoom (consumed by mmo_camera crate) ──────────────────────────
    pub zoom_in: Vec<UserBinding>,
    pub zoom_out: Vec<UserBinding>,
    pub zoom_speed: f32,

    // ── Sensitivity ─────────────────────────────────────────────────────────
    pub mouse_sensitivity: f32,
    pub gamepad_sensitivity: f32,
    pub kb_turn_speed: f32,
}

impl Default for MmoBindings {
    fn default() -> Self {
        use GamepadButton as Gp;
        use KeyCode::*;
        use MouseButton::*;
        Self {
            move_forward: vec![UserBinding::Key(KeyW), UserBinding::Key(ArrowUp)],
            move_backward: vec![UserBinding::Key(KeyS), UserBinding::Key(ArrowDown)],
            turn_left: vec![UserBinding::Key(KeyA), UserBinding::Key(ArrowLeft)],
            turn_right: vec![UserBinding::Key(KeyD), UserBinding::Key(ArrowRight)],
            strafe_left: vec![UserBinding::Key(KeyQ)],
            strafe_right: vec![UserBinding::Key(KeyE)],
            jump: vec![UserBinding::Key(Space)],
            mouse_look: vec![UserBinding::Mouse(Right)],
            left_mouse: vec![UserBinding::Mouse(Left)],
            auto_run: vec![UserBinding::Key(NumLock)],
            sprint: vec![],
            gamepad_jump: vec![UserBinding::Gamepad(Gp::South)],
            gamepad_sprint: vec![UserBinding::Gamepad(Gp::LeftThumb)],
            // Both zoom bindings use the scroll wheel; mmo_camera applies
            // Negate on the zoom_out action to reverse the scroll direction.
            zoom_in: vec![UserBinding::MouseWheel],
            zoom_out: vec![UserBinding::MouseWheel],
            zoom_speed: 2.0,
            mouse_sensitivity: 0.003,
            gamepad_sensitivity: 2.5,
            kb_turn_speed: std::f32::consts::PI,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bindings_serde_round_trip() {
        let b = MmoBindings::default();
        let json = serde_json::to_string(&b).unwrap();
        let restored: MmoBindings = serde_json::from_str(&json).unwrap();
        assert!((restored.mouse_sensitivity - b.mouse_sensitivity).abs() < 1e-6);
        assert_eq!(restored.move_forward.len(), b.move_forward.len());
    }

    #[test]
    fn user_binding_key_serde() {
        use bevy::input::keyboard::KeyCode;
        let binding = UserBinding::Key(KeyCode::KeyW);
        let json = serde_json::to_string(&binding).unwrap();
        let restored: UserBinding = serde_json::from_str(&json).unwrap();
        assert_eq!(binding, restored);
    }
}
