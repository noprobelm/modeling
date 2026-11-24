mod settings;

use bevy::prelude::*;
use leafwing_input_manager::prelude::*;

pub use self::settings::CameraSettings;
use crate::{AppSystems, settings::RegisterSettings};

/// Spawns the main camera and pans it with the keybindings in [`CameraSettings`].
pub(super) struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.register_settings::<CameraSettings>()
            .add_plugins(InputManagerPlugin::<CameraAction>::default())
            .add_systems(Startup, spawn_camera.spawn())
            .add_systems(
                Update,
                (
                    apply_bindings.run_if(
                        resource_changed::<CameraSettings>
                            .or_eager(any_match_filter::<Added<Camera2d>>),
                    ),
                    pan,
                )
                    .chain()
                    .in_set(AppSystems::Update),
            );
    }
}

#[derive(Actionlike, PartialEq, Eq, Hash, Clone, Copy, Debug, Reflect)]
#[allow(
    clippy::enum_variant_names,
    reason = "only panning so far; actions like zooming will follow"
)]
pub enum CameraAction {
    PanUp,
    PanLeft,
    PanDown,
    PanRight,
}

fn spawn_camera() -> impl Scene {
    bsn! {
        #Camera
        Camera2d
    }
}

/// Rebuilds each camera's [`InputMap`] from [`CameraSettings`], for new cameras and whenever the
/// bindings change.
fn apply_bindings(
    mut commands: Commands,
    settings: Res<CameraSettings>,
    cameras: Query<(Entity, Option<&InputMap<CameraAction>>), With<Camera2d>>,
) {
    let input_map = settings.input_map();
    for (camera, current) in &cameras {
        // `resource_changed` also fires on mutable access without a real change.
        if current != Some(&input_map) {
            commands.entity(camera).insert(input_map.clone());
        }
    }
}

fn pan(
    cameras: Query<(&mut Transform, &Projection, &ActionState<CameraAction>), With<Camera2d>>,
    settings: Res<CameraSettings>,
    // Real time, so the camera still moves while virtual time is paused or slowed.
    time: Res<Time<Real>>,
) {
    for (mut transform, projection, actions) in cameras {
        let direction = [
            (CameraAction::PanUp, Vec2::Y),
            (CameraAction::PanLeft, Vec2::NEG_X),
            (CameraAction::PanDown, Vec2::NEG_Y),
            (CameraAction::PanRight, Vec2::X),
        ]
        .into_iter()
        .filter(|(action, _)| actions.pressed(action))
        .map(|(_, direction)| direction)
        .sum::<Vec2>()
        .normalize_or_zero();
        if direction == Vec2::ZERO {
            continue;
        }

        // World units per screen pixel.
        let scale = match projection {
            Projection::Orthographic(orthographic) => orthographic.scale,
            _ => 1.0,
        };
        let delta = direction * settings.pan_speed * scale * time.delta_secs();
        transform.translation += delta.extend(0.0);
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::{input::InputPlugin, time::TimeUpdateStrategy};

    use super::*;

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, InputPlugin))
            .add_plugins(InputManagerPlugin::<CameraAction>::default())
            .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
                100,
            )))
            .init_resource::<CameraSettings>()
            .add_systems(Update, (apply_bindings, pan).chain());
        app.world_mut().spawn((
            Camera2d,
            Projection::Orthographic(OrthographicProjection {
                scale: 2.0,
                ..OrthographicProjection::default_2d()
            }),
        ));
        // `Time<Real>` records its starting point on the first update.
        app.update();
        app
    }

    fn camera_position(app: &mut App) -> Vec2 {
        app.world_mut()
            .query_filtered::<&Transform, With<Camera2d>>()
            .single(app.world())
            .unwrap()
            .translation
            .xy()
    }

    #[test]
    fn bound_keys_pan_scaled_by_zoom() {
        let mut app = app();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyD);
        app.update();
        // 500 px/s * scale 2 * 0.1 s.
        let moved = camera_position(&mut app);
        assert!((moved - Vec2::new(100.0, 0.0)).length() < 1e-3, "{moved}");
    }

    #[test]
    fn rebinding_takes_effect() {
        let mut app = app();
        app.world_mut().resource_mut::<CameraSettings>().pan_right = vec![KeyCode::KeyL.into()];
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::KeyD);
        app.update();
        assert_eq!(camera_position(&mut app), Vec2::ZERO, "old binding removed");

        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(KeyCode::KeyD);
        keys.press(KeyCode::KeyL);
        app.update();
        assert!(camera_position(&mut app).x > 0.0, "new binding works");
    }
}
