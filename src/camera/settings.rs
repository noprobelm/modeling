use bevy::prelude::*;
use leafwing_input_manager::prelude::InputMap;
use serde::{Deserialize, Serialize};

use super::CameraAction;
use crate::{input::InputButton, settings::Settings};

/// Camera panning speed and keybindings, saved as `[camera]`.
#[derive(Resource, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CameraSettings {
    /// Pan speed in screen pixels per second
    pub pan_speed: f32,
    pub pan_up: Vec<InputButton>,
    pub pan_left: Vec<InputButton>,
    pub pan_down: Vec<InputButton>,
    pub pan_right: Vec<InputButton>,
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            pan_speed: 500.0,
            pan_up: vec![KeyCode::KeyW.into(), KeyCode::ArrowUp.into()],
            pan_left: vec![KeyCode::KeyA.into(), KeyCode::ArrowLeft.into()],
            pan_down: vec![KeyCode::KeyS.into(), KeyCode::ArrowDown.into()],
            pan_right: vec![KeyCode::KeyD.into(), KeyCode::ArrowRight.into()],
        }
    }
}

impl Settings for CameraSettings {
    const SECTION: &'static str = "camera";
}

impl CameraSettings {
    /// The [`InputMap`] these bindings describe.
    pub fn input_map(&self) -> InputMap<CameraAction> {
        let mut input_map = InputMap::default();
        for (action, buttons) in [
            (CameraAction::PanUp, &self.pan_up),
            (CameraAction::PanLeft, &self.pan_left),
            (CameraAction::PanDown, &self.pan_down),
            (CameraAction::PanRight, &self.pan_right),
        ] {
            for button in buttons {
                button.insert_into(&mut input_map, action);
            }
        }
        input_map
    }
}

#[cfg(all(test, not(target_family = "wasm")))]
mod tests {
    use super::*;

    #[test]
    fn bindings_round_trip_through_toml() {
        let settings = CameraSettings {
            pan_up: vec![KeyCode::KeyW.into(), MouseButton::Middle.into()],
            ..default()
        };
        let text = toml::to_string(&settings).unwrap();
        assert!(
            text.contains(r#"pan_up = ["KeyW", "MouseMiddle"]"#),
            "{text}"
        );
        assert_eq!(toml::from_str::<CameraSettings>(&text).unwrap(), settings);
    }
}
