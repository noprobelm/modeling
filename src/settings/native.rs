//! Native storage: every section lives in one TOML file in the platform config directory.

use std::{
    any::type_name,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use bevy::{app::OnAppExitSystems, prelude::*};
use serde::{Serialize, de::DeserializeOwned};

use super::Settings;

/// Folder inside the platform config directory that holds the settings file.
const APP_DIR: &str = "modeling";
const FILE_NAME: &str = "settings.toml";

/// Delay between the first unsaved change and the write. Batches bursts of changes (dragging a
/// slider, say) into one write, and bounds what a crash can lose.
const SAVE_DELAY: Duration = Duration::from_secs(1);
/// Delay before retrying a write that failed.
const RETRY_DELAY: Duration = Duration::from_secs(10);

const HEADER: &str = "\
# Settings for Modeling. The app rewrites this file while it runs, so edit it only while
# the app is closed. Delete a [section] (or the whole file) to restore its defaults.
";

pub(super) fn build(app: &mut App) {
    build_with(
        app,
        dirs::config_dir().map(|dir| dir.join(APP_DIR).join(FILE_NAME)),
    );
}

/// [`build`] with an explicit file location. `None` keeps settings in memory only.
fn build_with(app: &mut App, path: Option<PathBuf>) {
    app.insert_resource(SettingsStore::open(path))
        .configure_sets(
            Last,
            (SettingsSystems::Stage, SettingsSystems::Save).chain(),
        )
        .add_systems(
            Last,
            (
                save_when_due,
                // Closing the window sends `AppExit` from `Last`; `OnAppExitSystems` runs after it.
                save_on_exit
                    .run_if(on_message::<AppExit>)
                    .in_set(OnAppExitSystems),
            )
                .in_set(SettingsSystems::Save),
        );
}

pub(super) fn register<T: Settings>(app: &mut App) {
    let settings = app
        .world_mut()
        .get_resource_mut::<SettingsStore>()
        .unwrap_or_else(|| {
            panic!(
                "`SettingsPlugin` must be added before registering settings (registering {})",
                type_name::<T>()
            )
        })
        .claim::<T>();
    app.insert_resource(settings).add_systems(
        Last,
        stage_changes::<T>
            .run_if(resource_changed::<T>)
            .in_set(SettingsSystems::Stage),
    );
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
enum SettingsSystems {
    /// Copy changed settings into the document.
    Stage,
    /// Write the document out.
    Save,
}

/// In-memory copy of the settings file.
#[derive(Resource)]
struct SettingsStore {
    /// Where to save. `None` when there's no config directory, or when the file found at
    /// startup couldn't be read or backed up, and so mustn't be overwritten.
    path: Option<PathBuf>,
    /// The whole file. Sections no registered type claims are kept and written back unchanged.
    document: toml::Table,
    /// When the next write is due, as `Time<Real>` elapsed time. `None` when nothing's unsaved.
    save_at: Option<Duration>,
    /// Whether the file found at startup has been backed up yet. Happens at most once.
    backed_up: bool,
}

impl SettingsStore {
    /// Reads the file at `path`. Never fails: problems are logged, and at worst every section
    /// starts from its defaults.
    fn open(path: Option<PathBuf>) -> Self {
        let mut store = Self {
            path,
            document: default(),
            save_at: None,
            backed_up: false,
        };
        let Some(path) = store.path.clone() else {
            warn!("No config directory found; settings won't be saved this session.");
            return store;
        };
        match fs::read(&path) {
            Ok(bytes) => match toml::from_slice(&bytes) {
                Ok(document) => {
                    info!("Loaded settings from {}", path.display());
                    store.document = document;
                }
                Err(err) => {
                    warn!(
                        "{} isn't valid TOML, so every section starts from its defaults.\n{err}",
                        path.display()
                    );
                    store.back_up();
                }
            },
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                info!(
                    "No settings file yet; it will be created at {}",
                    path.display()
                );
            }
            Err(err) => {
                warn!(
                    "Couldn't read {}: {err}. Settings won't be saved this session.",
                    path.display()
                );
                store.path = None;
            }
        }
        store
    }

    /// Loads `T` from its section.
    fn claim<T: Settings>(&mut self) -> T {
        let section = T::SECTION;
        let name = type_name::<T>();
        if let Err(err) = from_table::<T>(toml::Table::new()) {
            panic!(
                "{name} must load from an empty [{section}] section, so that adding a field \
                 doesn't invalidate existing files. Add #[serde(default)] to it. ({err})"
            );
        }

        let settings = match self.document.get(section).cloned() {
            None => T::default(),
            Some(toml::Value::Table(stored)) => {
                let (settings, rejected) = load_section(stored);
                if !rejected.is_empty() {
                    warn!(
                        "Invalid values in [{section}] were reset to their defaults: {}",
                        rejected.join(", ")
                    );
                    self.back_up();
                }
                settings
            }
            Some(other) => {
                warn!(
                    "[{section}] should be a table, not a {}; using its defaults.",
                    other.type_str()
                );
                self.back_up();
                T::default()
            }
        };
        // Writes the section back if it was missing, incomplete, or repaired.
        self.stage(section, &settings, Duration::ZERO);
        settings
    }

    /// Copies `settings` into the document, and schedules a save if its section changed.
    fn stage(&mut self, section: &'static str, settings: &impl Serialize, now: Duration) {
        let table = match to_table(settings) {
            Ok(table) => toml::Value::Table(table),
            Err(err) => {
                error!("Couldn't serialize settings section [{section}]: {err}");
                return;
            }
        };
        if self.document.get(section) != Some(&table) {
            self.document.insert(section.to_owned(), table);
            self.save_at.get_or_insert(now + SAVE_DELAY);
        }
    }

    /// Writes the document if anything is unsaved. Returns whether the file is now up to date.
    fn save(&mut self) -> bool {
        if self.save_at.is_none() {
            return true;
        }
        let Some(path) = &self.path else {
            // Nowhere to save to; settings stay in memory.
            self.save_at = None;
            return true;
        };
        let result = toml::to_string(&self.document)
            .map_err(io::Error::other)
            .and_then(|body| write_atomically(path, &format!("{HEADER}\n{body}")));
        match result {
            Ok(()) => {
                debug!("Saved settings to {}", path.display());
                self.save_at = None;
                true
            }
            Err(err) => {
                warn!("Couldn't save settings to {}: {err}", path.display());
                false
            }
        }
    }

    /// Copies the file found at startup aside before it's rewritten with repaired contents.
    /// If that fails, saving is turned off for the session so the original is never lost.
    fn back_up(&mut self) {
        if self.backed_up {
            return;
        }
        let Some(path) = &self.path else {
            return;
        };
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| since.as_secs());
        let backup = path.with_extension(format!("toml.{stamp}.bak"));
        match fs::copy(path, &backup) {
            Ok(_) => {
                warn!("Backed up the original settings to {}", backup.display());
                self.backed_up = true;
            }
            Err(err) => {
                error!(
                    "Couldn't back up {} ({err}); settings won't be saved this session.",
                    path.display()
                );
                self.path = None;
            }
        }
    }
}

impl Drop for SettingsStore {
    /// Last-chance save for exits that skip [`save_on_exit`]. Skipped while panicking.
    fn drop(&mut self) {
        if !std::thread::panicking() {
            self.save();
        }
    }
}

fn stage_changes<T: Settings>(
    settings: Res<T>,
    mut store: ResMut<SettingsStore>,
    time: Res<Time<Real>>,
) {
    store.stage(T::SECTION, &*settings, time.elapsed());
}

fn save_when_due(mut store: ResMut<SettingsStore>, time: Res<Time<Real>>) {
    let now = time.elapsed();
    if store.save_at.is_some_and(|due| now >= due) && !store.save() {
        store.save_at = Some(now + RETRY_DELAY);
    }
}

fn save_on_exit(mut store: ResMut<SettingsStore>) {
    store.save();
}

/// Loads `T` from a stored section, keeping every value that deserializes. Keys whose values
/// don't are left at their defaults, and returned.
fn load_section<T: Settings>(stored: toml::Table) -> (T, Vec<String>) {
    if let Ok(settings) = from_table(stored.clone()) {
        return (settings, Vec::new());
    }
    let mut valid = toml::Table::new();
    let mut rejected = Vec::new();
    for (key, value) in stored {
        valid.insert(key.clone(), value);
        if from_table::<T>(valid.clone()).is_err() {
            valid.remove(&key);
            rejected.push(key);
        }
    }
    (from_table(valid).unwrap_or_default(), rejected)
}

fn from_table<T: DeserializeOwned>(table: toml::Table) -> Result<T, toml::de::Error> {
    toml::Value::Table(table).try_into()
}

/// Serializes via TOML text rather than `toml::Value::try_from`, which widens `f32` to `f64`
/// and would save `0.1_f32` as `0.10000000149011612`.
fn to_table(value: &impl Serialize) -> Result<toml::Table, String> {
    let text = toml::to_string(value).map_err(|err| err.to_string())?;
    toml::from_str(&text).map_err(|err| err.to_string())
}

/// Writes through a temporary file and a rename, so a crash mid-write can't leave a truncated
/// settings file behind.
fn write_atomically(path: &Path, contents: &str) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let temp = path.with_extension("toml.tmp");
    let result = write_synced(&temp, contents).and_then(|()| fs::rename(&temp, path));
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

fn write_synced(path: &Path, contents: &str) -> io::Result<()> {
    let mut file = fs::File::create(path)?;
    file.write_all(contents.as_bytes())?;
    file.sync_all()
}

#[cfg(test)]
mod tests {
    use bevy::time::TimeUpdateStrategy;
    use serde::Deserialize;
    use tempfile::TempDir;

    use super::*;
    use crate::settings::RegisterSettings;

    #[derive(Resource, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(default)]
    struct Demo {
        count: i64,
        ratio: f32,
        label: String,
    }

    impl Default for Demo {
        fn default() -> Self {
            Self {
                count: 1,
                ratio: 0.5,
                label: "default".into(),
            }
        }
    }

    impl Settings for Demo {
        const SECTION: &'static str = "demo";
    }

    /// A settings path in a fresh temporary directory, optionally pre-filled.
    fn settings_file(contents: Option<&str>) -> (TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(FILE_NAME);
        if let Some(contents) = contents {
            fs::write(&path, contents).unwrap();
        }
        (dir, path)
    }

    /// An app that registers `Demo` and saves to `path`. Its clock advances by `SAVE_DELAY`
    /// each update, so a pending save happens on the next update.
    fn app_saving_to(path: &Path) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(TimeUpdateStrategy::ManualDuration(SAVE_DELAY));
        build_with(&mut app, Some(path.to_owned()));
        app.register_settings::<Demo>();
        // `Time<Real>` only records its starting point on the first update, then advances.
        app.update();
        app
    }

    /// Stops the clock, so only exiting can trigger a save.
    fn freeze_clock(app: &mut App) {
        app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
    }

    fn demo(app: &App) -> Demo {
        app.world().resource::<Demo>().clone()
    }

    fn backups(dir: &TempDir) -> Vec<PathBuf> {
        fs::read_dir(dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "bak"))
            .collect()
    }

    #[test]
    fn first_run_writes_defaults() {
        let (dir, path) = settings_file(None);
        let mut app = app_saving_to(&path);
        assert_eq!(demo(&app), Demo::default());

        app.update();
        let saved = fs::read_to_string(&path).unwrap();
        assert!(
            saved.ends_with("[demo]\ncount = 1\nratio = 0.5\nlabel = \"default\"\n"),
            "{saved}"
        );
        assert!(backups(&dir).is_empty());
    }

    #[test]
    fn changes_are_saved_and_reloaded() {
        let (_dir, path) = settings_file(None);
        let mut app = app_saving_to(&path);
        app.world_mut().resource_mut::<Demo>().ratio = 0.1;

        app.update();
        let saved = fs::read_to_string(&path).unwrap();
        assert!(
            saved.contains("ratio = 0.1\n"),
            "f32 saved as typed:\n{saved}"
        );
        assert_eq!(demo(&app_saving_to(&path)).ratio, 0.1);
    }

    #[test]
    fn exiting_saves_immediately() {
        let (_dir, path) = settings_file(None);
        let mut app = app_saving_to(&path);
        freeze_clock(&mut app);
        app.world_mut().resource_mut::<Demo>().count = 7;
        app.world_mut().write_message(AppExit::Success);

        app.update();
        assert!(fs::read_to_string(&path).unwrap().contains("count = 7"));
    }

    #[test]
    fn dropping_the_app_saves_pending_changes() {
        let (_dir, path) = settings_file(None);
        let mut app = app_saving_to(&path);
        freeze_clock(&mut app);
        app.world_mut().resource_mut::<Demo>().count = 9;
        app.update();
        assert!(!path.exists());

        drop(app);
        assert!(fs::read_to_string(&path).unwrap().contains("count = 9"));
    }

    #[test]
    fn unchanged_values_are_not_resaved() {
        let (_dir, path) = settings_file(None);
        app_saving_to(&path).update();

        let mut app = app_saving_to(&path);
        assert_eq!(app.world().resource::<SettingsStore>().save_at, None);
        // Mutable access without a real change, as UI widgets do every frame.
        app.world_mut().resource_mut::<Demo>().set_changed();
        app.update();
        assert_eq!(app.world().resource::<SettingsStore>().save_at, None);
    }

    #[test]
    fn missing_fields_take_their_defaults() {
        let (dir, path) = settings_file(Some("[demo]\nlabel = \"mine\"\n"));
        let mut app = app_saving_to(&path);
        let expected = Demo {
            label: "mine".into(),
            ..default()
        };
        assert_eq!(demo(&app), expected);
        assert!(backups(&dir).is_empty(), "a missing field isn't damage");

        app.update();
        assert!(fs::read_to_string(&path).unwrap().contains("count = 1\n"));
    }

    #[test]
    fn invalid_values_reset_individually_after_a_backup() {
        let original = "[demo]\ncount = \"seven\"\nlabel = \"mine\"\n";
        let (dir, path) = settings_file(Some(original));
        let mut app = app_saving_to(&path);
        let expected = Demo {
            label: "mine".into(),
            ..default()
        };
        assert_eq!(demo(&app), expected);
        let backups = backups(&dir);
        assert_eq!(backups.len(), 1);
        assert_eq!(fs::read_to_string(&backups[0]).unwrap(), original);

        app.update();
        let saved = fs::read_to_string(&path).unwrap();
        assert!(
            saved.contains("count = 1\n") && saved.contains("label = \"mine\"\n"),
            "{saved}"
        );
    }

    #[test]
    fn unparseable_files_are_backed_up_and_replaced() {
        let original = "[demo\ncount = ";
        let (dir, path) = settings_file(Some(original));
        let mut app = app_saving_to(&path);
        assert_eq!(demo(&app), Demo::default());
        assert_eq!(fs::read_to_string(&backups(&dir)[0]).unwrap(), original);

        app.update();
        assert!(fs::read_to_string(&path).unwrap().contains("[demo]\n"));
    }

    #[test]
    fn unclaimed_sections_are_kept() {
        let (_dir, path) = settings_file(Some("[from_a_newer_version]\nkeep = true\n"));
        app_saving_to(&path).update();

        let saved = fs::read_to_string(&path).unwrap();
        assert!(
            saved.contains("[from_a_newer_version]\nkeep = true\n") && saved.contains("[demo]\n"),
            "{saved}"
        );
    }

    #[test]
    #[should_panic(expected = "used by more than one type")]
    fn sections_must_be_unique() {
        #[derive(Resource, Default, Serialize, Deserialize)]
        #[serde(default)]
        struct Imposter {
            count: i64,
        }
        impl Settings for Imposter {
            const SECTION: &'static str = "demo";
        }

        let (_dir, path) = settings_file(None);
        app_saving_to(&path).register_settings::<Imposter>();
    }

    #[test]
    #[should_panic(expected = "#[serde(default)]")]
    fn settings_must_tolerate_missing_fields() {
        #[derive(Resource, Default, Serialize, Deserialize)]
        struct Strict {
            count: i64,
        }
        impl Settings for Strict {
            const SECTION: &'static str = "strict";
        }

        let (_dir, path) = settings_file(None);
        app_saving_to(&path).register_settings::<Strict>();
    }

    #[test]
    #[should_panic(expected = "`SettingsPlugin` must be added")]
    fn registering_requires_the_plugin() {
        App::new().register_settings::<Demo>();
    }
}
