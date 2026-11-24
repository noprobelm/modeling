//! Web storage: nothing is persisted, so every section starts from its default each session.

use bevy::prelude::*;

use super::Settings;

pub(super) fn build(_app: &mut App) {}

pub(super) fn register<T: Settings>(app: &mut App) {
    app.insert_resource(T::default());
}
