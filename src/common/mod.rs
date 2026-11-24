use bevy::prelude::*;

pub(super) struct CommonPlugin;

impl Plugin for CommonPlugin {
    fn build(&self, app: &mut App) {}
}

#[derive(Component, Debug, Copy, Clone)]
pub struct Velocity2d {
    pub velocity: Vec2,
}

impl Velocity2d {
    pub fn new(x: f32, y: f32) -> Self {
        Velocity2d {
            velocity: Vec2::new(x, y),
        }
    }

    pub const fn zero() -> Self {
        Velocity2d {
            velocity: Vec2::ZERO,
        }
    }
}

#[derive(Component, Debug, Copy, Clone)]
pub struct Acceleration2d {
    pub acceleration: Vec2,
}

impl Acceleration2d {
    pub fn new(x: f32, y: f32) -> Self {
        Acceleration2d {
            acceleration: Vec2::new(x, y),
        }
    }

    pub const fn zero() -> Self {
        Acceleration2d {
            acceleration: Vec2::ZERO,
        }
    }
}
