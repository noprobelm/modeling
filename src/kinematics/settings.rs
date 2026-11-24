use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::settings::Settings;

/// Initial conditions for 2D motion under constant acceleration, saved as `[kinematics_2d]`:
///
/// `x(t) = x_0 + v_0x·t + ½·a_x·t²` and `y(t) = y_0 + v_0y·t + ½·a_y·t²`.
#[derive(Resource, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Kinematics2dSettings {
    /// Initial x position.
    pub x_0: f32,
    /// Initial y position.
    pub y_0: f32,
    /// Initial x velocity.
    pub v_0x: f32,
    /// Initial y velocity.
    pub v_0y: f32,
    /// Constant x acceleration.
    pub a_x: f32,
    /// Constant y acceleration.
    pub a_y: f32,
}

impl Default for Kinematics2dSettings {
    fn default() -> Self {
        Self {
            x_0: 0.0,
            y_0: 0.0,
            v_0x: 0.0,
            v_0y: 0.0,
            a_x: 0.0,
            a_y: 0.0,
        }
    }
}

impl Settings for Kinematics2dSettings {
    const SECTION: &'static str = "kinematics_2d";
}
