use bevy::prelude::Vec2;
use bevy_enhanced_input::prelude::*;

#[derive(InputAction)]
#[action_output(bool)]
pub(crate) struct MoveForward;

#[derive(InputAction)]
#[action_output(bool)]
pub(crate) struct MoveBackward;

#[derive(InputAction)]
#[action_output(bool)]
pub(crate) struct TurnLeft;

#[derive(InputAction)]
#[action_output(bool)]
pub(crate) struct TurnRight;

#[derive(InputAction)]
#[action_output(bool)]
pub(crate) struct StrafeLeft;

#[derive(InputAction)]
#[action_output(bool)]
pub(crate) struct StrafeRight;

#[derive(InputAction)]
#[action_output(bool)]
pub(crate) struct Jump;

#[derive(InputAction)]
#[action_output(bool)]
pub(crate) struct MouseLook;

#[derive(InputAction)]
#[action_output(bool)]
pub(crate) struct LeftMouse;

#[derive(InputAction)]
#[action_output(bool)]
pub(crate) struct AutoRun;

#[derive(InputAction)]
#[action_output(bool)]
pub(crate) struct Sprint;

#[derive(InputAction)]
#[action_output(Vec2)]
pub(crate) struct MouseDelta;

#[derive(InputAction)]
#[action_output(Vec2)]
pub(crate) struct GamepadMove;

#[derive(InputAction)]
#[action_output(Vec2)]
pub(crate) struct GamepadCamera;
