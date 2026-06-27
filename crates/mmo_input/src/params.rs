use bevy::prelude::*;

#[derive(Component, Reflect, Clone)]
#[reflect(Component)]
pub struct MmoMovementParams {
    /// Base walk speed in world units per second.
    pub walk_speed: f32,
    /// Multiplier applied on top of walk_speed. Set by buffs/debuffs/mounts.
    pub speed_multiplier: f32,
    /// Tnua hover height above ground (centre of mass).
    pub float_height: f32,
    /// Horizontal acceleration on ground (units/s²).
    pub acceleration: f32,
    /// Horizontal acceleration in air (units/s²).
    pub air_acceleration: f32,
    /// Seconds after leaving a ledge during which jumping is still allowed.
    pub coyote_time: f32,
}

impl Default for MmoMovementParams {
    fn default() -> Self {
        Self {
            walk_speed: 7.0,
            speed_multiplier: 1.0,
            float_height: 0.5,
            acceleration: 60.0,
            air_acceleration: 10.0,
            coyote_time: 0.15,
        }
    }
}
