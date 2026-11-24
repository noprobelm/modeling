use bevy::prelude::*;

pub(super) struct CommonPlugin;

impl Plugin for CommonPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup);
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Sprite::from_image(
        asset_server.load("textures/2d_ball.png"),
    ));
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
