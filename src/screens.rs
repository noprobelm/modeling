use bevy::prelude::*;

pub(super) struct ScreenPlugin;

impl Plugin for ScreenPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<Screen>();
    }
}

#[derive(States, Copy, Clone, Debug, Default, Hash, Eq, PartialEq)]
pub enum Screen {
    /// Waiting for shared assets (see `common::TextureMapper`) to finish loading.
    #[default]
    Loading,
    Kinematics2d,
}
