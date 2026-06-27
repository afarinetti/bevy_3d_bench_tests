use bevy::prelude::*;
use bitflags::bitflags;
use serde::{Deserialize, Serialize};

#[derive(Component, Reflect, Serialize, Deserialize, Default, Clone)]
#[reflect(Component)]
pub struct MmoMovementState {
    /// Camera-relative movement intent. X = strafe (right+), Y = forward (+).
    /// Keyboard: 0.0 or ±1.0. Gamepad left stick: analog ±1.0.
    pub move_intent: Vec2,
    /// Vertical movement intent for swimming/flying. +1 = up, -1 = down.
    pub vertical_intent: f32,
    /// Character yaw change this frame in radians.
    pub yaw_delta: f32,
    /// Raw camera orbit delta (radians). X = yaw, Y = pitch. For mmo_camera.
    pub camera_delta: Vec2,
    /// Jump binding fired this frame. Signal only.
    pub jump: bool,
    /// Sprint binding fired this frame. Signal only.
    pub sprint: bool,
    /// Auto-run toggled on.
    pub auto_run: bool,
    /// RMB held (kb/mouse) or gamepad active (always true for Gamepad source).
    pub is_mouse_look: bool,
    /// Which input device drove this frame.
    pub input_source: InputSource,
    /// Monotonic tick counter for netcode / prediction.
    pub tick: u64,
}

#[derive(Reflect, Serialize, Deserialize, Clone, Copy, Default, PartialEq, Eq, Debug)]
pub enum InputSource {
    #[default]
    KeyboardMouse,
    Gamepad,
}

bitflags! {
    #[derive(Component, Reflect, Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
    #[reflect(opaque)]
    #[reflect(Component)]
    pub struct MmoMovementMask: u8 {
        /// Controls move_intent.
        const MOVEMENT = 0b0001;
        /// Controls yaw_delta only — camera_delta is never masked.
        const TURNING  = 0b0010;
        /// Controls jump and sprint signals.
        const JUMPING  = 0b0100;
        /// Controls vertical_intent.
        const VERTICAL = 0b1000;
        const ALL      = 0b1111;
    }
}

impl Default for MmoMovementMask {
    fn default() -> Self {
        Self::ALL
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn movement_state_serde_round_trip() {
        let state = MmoMovementState {
            move_intent: Vec2::new(0.5, -1.0),
            vertical_intent: 0.3,
            yaw_delta: 0.01,
            camera_delta: Vec2::new(0.02, -0.01),
            jump: true,
            sprint: false,
            auto_run: true,
            is_mouse_look: false,
            input_source: InputSource::Gamepad,
            tick: 42,
        };
        let json = serde_json::to_string(&state).unwrap();
        let restored: MmoMovementState = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.tick, 42);
        assert_eq!(restored.input_source, InputSource::Gamepad);
        assert!(restored.jump);
        assert!((restored.move_intent.x - 0.5).abs() < 1e-6);
    }

    #[test]
    fn movement_mask_bitflag_operations() {
        let mut mask = MmoMovementMask::ALL;
        assert!(mask.contains(MmoMovementMask::MOVEMENT));
        mask.remove(MmoMovementMask::MOVEMENT | MmoMovementMask::JUMPING);
        assert!(!mask.contains(MmoMovementMask::MOVEMENT));
        assert!(!mask.contains(MmoMovementMask::JUMPING));
        assert!(mask.contains(MmoMovementMask::TURNING));
        assert!(mask.contains(MmoMovementMask::VERTICAL));
    }

    #[test]
    fn empty_mask_has_no_bits() {
        assert_eq!(MmoMovementMask::empty().bits(), 0u8);
    }
}
