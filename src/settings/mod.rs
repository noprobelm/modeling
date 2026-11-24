//! Settings that persist between sessions.
//!
//! A settings type is a plain Bevy [`Resource`] saved as one `[section]` of `settings.toml`.
//! Registering it loads the saved value (or its [`Default`]). From then on, systems just use
//! `Res<T>` / `ResMut<T>`, and changes are written back automatically, about a second after
//! they happen and again on exit. There is no per-setting load or save code.
//!
//! # Adding settings
//!
//! ```ignore
//! use serde::{Deserialize, Serialize};
//!
//! use crate::settings::{RegisterSettings, Settings};
//!
//! #[derive(Resource, Debug, Clone, PartialEq, Serialize, Deserialize)]
//! #[serde(default)]
//! pub struct PendulumSettings {
//!     pub length: f64,
//!     pub damping: f64,
//! }
//!
//! impl Default for PendulumSettings {
//!     fn default() -> Self {
//!         Self { length: 1.0, damping: 0.0 }
//!     }
//! }
//!
//! impl Settings for PendulumSettings {
//!     const SECTION: &'static str = "pendulum";
//! }
//!
//! // In the owning plugin's `build`. That plugin must be added after `SettingsPlugin`.
//! app.register_settings::<PendulumSettings>();
//! ```
//!
//! - **New field:** add it with a default. Older files still load, and gain the field on the
//!   next save.
//! - **Renamed field:** `#[serde(alias = "old_name")]` keeps older files readable.
//! - **Nested structs:** give them `#[serde(default)]` too, so a partly written table keeps the
//!   values it does have.
//! - **Reacting to a setting** (applying it to a window, an input map, ...): a system with
//!   `.run_if(resource_changed::<T>)` handles both the loaded value and later changes.
//!
//! # Storage
//!
//! Native builds keep `settings.toml` in a `modeling` folder inside the platform config
//! directory: `$XDG_CONFIG_HOME` or `~/.config` on Linux, `~/Library/Application Support` on
//! macOS, `%APPDATA%` on Windows. A damaged file never stops the app from starting: values that
//! can't be read fall back to their defaults, and the original file is backed up beside it first.
//!
//! Web builds persist nothing; every section starts from its default each session.

#[cfg(not(target_family = "wasm"))]
mod native;
#[cfg(not(target_family = "wasm"))]
use native as backend;

#[cfg(target_family = "wasm")]
mod web;
#[cfg(target_family = "wasm")]
use web as backend;

use std::{any::type_name, collections::HashSet};

use bevy::prelude::*;
use serde::{Serialize, de::DeserializeOwned};

/// A group of settings saved as one `[section]` of `settings.toml`.
///
/// See the [module docs](crate::settings) for how to define and register one.
pub trait Settings: Resource + Default + Serialize + DeserializeOwned {
    /// The `[section]` name in `settings.toml`. Must be unique across the app.
    const SECTION: &'static str;
}

/// Loads `settings.toml` and keeps it in sync with every registered [`Settings`] type.
///
/// Add it before any plugin that calls [`RegisterSettings::register_settings`].
pub struct SettingsPlugin;

impl Plugin for SettingsPlugin {
    fn build(&self, app: &mut App) {
        backend::build(app);
    }
}

/// Registers [`Settings`] types on an [`App`].
pub trait RegisterSettings {
    /// Inserts `T` as a resource, loaded from its section of `settings.toml` (or
    /// `T::default()`), and saves it whenever it changes.
    ///
    /// # Panics
    ///
    /// If another type already uses `T::SECTION`. On native builds, also if [`SettingsPlugin`]
    /// hasn't been added yet, or if `T` can't load from an empty section (it's missing
    /// `#[serde(default)]`).
    fn register_settings<T: Settings>(&mut self) -> &mut Self;
}

impl RegisterSettings for App {
    fn register_settings<T: Settings>(&mut self) -> &mut Self {
        let unclaimed = self
            .world_mut()
            .get_resource_or_init::<RegisteredSections>()
            .0
            .insert(T::SECTION);
        assert!(
            unclaimed,
            "settings section [{}] is used by more than one type (the second is {})",
            T::SECTION,
            type_name::<T>()
        );
        backend::register::<T>(self);
        self
    }
}

/// Every section registered so far, to catch two types sharing one.
#[derive(Resource, Default)]
struct RegisteredSections(HashSet<&'static str>);
