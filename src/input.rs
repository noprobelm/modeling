//! Input binding types shared by every display's keybinding settings.

use std::fmt;

use bevy::prelude::*;
use leafwing_input_manager::prelude::{Actionlike, InputMap};
use serde::{Deserialize, Serialize, de::IntoDeserializer};

/// A keyboard key or mouse button that can be bound to an action and saved in `settings.toml`.
///
/// Saved as a plain string: keys by their [`KeyCode`] name (`"KeyW"`, `"ArrowUp"`, `"Space"`),
/// mouse buttons as `"MouseLeft"`, `"MouseRight"`, `"MouseMiddle"`, `"MouseBack"`,
/// `"MouseForward"`, or `"Mouse<n>"` for other buttons.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(into = "String", try_from = "String")]
pub enum InputButton {
    Key(KeyCode),
    Mouse(MouseButton),
}

impl InputButton {
    /// Binds this button to `action` in `input_map`, alongside any existing bindings.
    pub fn insert_into<A: Actionlike>(self, input_map: &mut InputMap<A>, action: A) {
        match self {
            Self::Key(key) => input_map.insert(action, key),
            Self::Mouse(button) => input_map.insert(action, button),
        };
    }
}

const MOUSE_PREFIX: &str = "Mouse";

impl fmt::Display for InputButton {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // Unit variants debug-print as their serde names.
            Self::Key(key) => write!(f, "{key:?}"),
            Self::Mouse(MouseButton::Other(n)) => write!(f, "{MOUSE_PREFIX}{n}"),
            Self::Mouse(button) => write!(f, "{MOUSE_PREFIX}{button:?}"),
        }
    }
}

impl From<InputButton> for String {
    fn from(button: InputButton) -> Self {
        button.to_string()
    }
}

impl TryFrom<String> for InputButton {
    type Error = String;

    fn try_from(name: String) -> Result<Self, Self::Error> {
        if let Some(button) = name.strip_prefix(MOUSE_PREFIX) {
            let button = match button {
                "Left" => MouseButton::Left,
                "Right" => MouseButton::Right,
                "Middle" => MouseButton::Middle,
                "Back" => MouseButton::Back,
                "Forward" => MouseButton::Forward,
                other => MouseButton::Other(
                    other
                        .parse()
                        .map_err(|_| format!("unknown mouse button {name:?}"))?,
                ),
            };
            return Ok(Self::Mouse(button));
        }
        let deserializer: serde::de::value::StrDeserializer<'_, serde::de::value::Error> =
            name.as_str().into_deserializer();
        KeyCode::deserialize(deserializer)
            .map(Self::Key)
            .map_err(|_| format!("unknown key {name:?}"))
    }
}

impl From<KeyCode> for InputButton {
    fn from(key: KeyCode) -> Self {
        Self::Key(key)
    }
}

impl From<MouseButton> for InputButton {
    fn from(button: MouseButton) -> Self {
        Self::Mouse(button)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_round_trip() {
        for button in [
            InputButton::Key(KeyCode::KeyW),
            InputButton::Key(KeyCode::ArrowUp),
            InputButton::Mouse(MouseButton::Left),
            InputButton::Mouse(MouseButton::Other(7)),
        ] {
            assert_eq!(InputButton::try_from(button.to_string()), Ok(button));
        }
        assert_eq!(
            InputButton::Mouse(MouseButton::Other(7)).to_string(),
            "Mouse7"
        );
        assert!(InputButton::try_from("KeyNope".to_owned()).is_err());
        assert!(InputButton::try_from("MouseNope".to_owned()).is_err());
    }
}
