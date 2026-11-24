use bevy::{asset::LoadState, prelude::*};

use crate::screens::Screen;

pub(super) struct CommonPlugin;

impl Plugin for CommonPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(Update, wait_for_textures.run_if(in_state(Screen::Loading)));
    }
}

#[derive(Resource, Clone, Debug, Default)]
pub struct TextureMapper {
    pub ball_2d_128_128: Handle<Image>,
}

impl TextureMapper {
    /// Every texture handle held by the mapper. This method is used to determine if our assets have
    /// all loaded
    fn ids(&self) -> impl Iterator<Item = AssetId<Image>> {
        [self.ball_2d_128_128.id()].into_iter()
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let ball_2d_128_128_handle = asset_server.load("textures/ball_2d_128_128.png");

    commands.insert_resource(TextureMapper {
        ball_2d_128_128: ball_2d_128_128_handle,
    });
}

/// Leaves [`Screen::Loading`] once every texture in [`TextureMapper`] has loaded.
fn wait_for_textures(
    textures: Res<TextureMapper>,
    asset_server: Res<AssetServer>,
    mut next_screen: ResMut<NextState<Screen>>,
) {
    let mut all_loaded = true;
    for id in textures.ids() {
        if let LoadState::Failed(err) = asset_server.load_state(id) {
            error_once!("Failed to load texture {id:?}: {err}");
        }
        all_loaded &= asset_server.is_loaded_with_dependencies(id);
    }

    if all_loaded {
        next_screen.set(Screen::Kinematics2d);
    }
}

/// Position in meters. The source of truth for where a simulated object is; its [`Transform`]
/// (in pixels) is derived from it using the display's pixels-per-meter scale.
#[derive(Component, Debug, Copy, Clone)]
#[require(Transform)]
pub struct Position2d {
    pub position: Vec2,
}

impl Position2d {
    pub fn new(x: f32, y: f32) -> Self {
        Position2d {
            position: Vec2::new(x, y),
        }
    }

    pub const fn zero() -> Self {
        Position2d {
            position: Vec2::ZERO,
        }
    }
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
