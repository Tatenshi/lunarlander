use crate::vecmath::Vec2d;

use super::{entity::Entity, BorderBehavior, MAX_ACCELERATION, VELOCITY_MISSILE};

#[derive(Clone, PartialEq, Default)]
pub struct Missile {
    pub entity_id: usize,
    pub time_to_live: f32, // in seconds
    pub hostile: bool,
}

impl Missile {
    pub fn new(
        id: usize,
        entity: &mut Entity,
        pos: Vec2d,
        direction: Vec2d,
        hostile: bool,
    ) -> Self {
        entity.set_position(pos);
        entity.set_direction(direction);
        entity.set_acceleration(direction * MAX_ACCELERATION);
        entity.set_max_velocity(VELOCITY_MISSILE);
        entity.set_border_behavior(BorderBehavior::Dismiss);
        entity.set_angle(direction.angle_360());

        Self {
            entity_id: id,
            time_to_live: 5.0f32,
            hostile: hostile,
        }
    }
}
