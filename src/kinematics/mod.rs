mod settings;

use bevy::prelude::*;

pub use self::settings::Kinematics2dSettings;
use crate::{
    Acceleration2d, Position2d, TextureMapper, Velocity2d, screens::Screen,
    settings::RegisterSettings,
};

pub(super) struct KinematicsPlugin;

impl Plugin for KinematicsPlugin {
    fn build(&self, app: &mut App) {
        app.register_settings::<Kinematics2dSettings>()
            .add_sub_state::<SimulationState>()
            .init_resource::<PixelsPerMeter>()
            .add_systems(OnEnter(Screen::Kinematics2d), setup)
            .add_systems(
                FixedUpdate,
                solve.run_if(in_state(SimulationState::Running)),
            )
            .add_systems(
                PostUpdate,
                (
                    sync_transforms.before(TransformSystems::Propagate),
                    sync_camera.after(sync_transforms),
                ),
            );
    }
}

/// Screen pixels per simulated meter.
/// [`sync_transforms`] converts to pixels.
#[derive(Resource, Copy, Clone, Debug)]
pub struct PixelsPerMeter(pub f32);

impl Default for PixelsPerMeter {
    fn default() -> Self {
        Self(100.0)
    }
}

#[derive(SubStates, Copy, Clone, Eq, PartialEq, Hash, Debug, Default)]
#[source(Screen = Screen::Kinematics2d)]
enum SimulationState {
    #[default]
    Running,
    Paused,
}

#[derive(Component, Debug, Copy, Clone)]
pub struct PointObject;

#[derive(Component, Debug, Copy, Clone, Deref, DerefMut)]
pub struct VelocityInitial(pub Velocity2d);

impl VelocityInitial {
    pub fn new(x: f32, y: f32) -> Self {
        Self(Velocity2d::new(x, y))
    }

    pub const fn zero() -> Self {
        Self(Velocity2d::zero())
    }
}

#[derive(Component, Debug, Copy, Clone, Deref, DerefMut)]
pub struct AccelerationInitial(pub Acceleration2d);

impl AccelerationInitial {
    pub fn new(x: f32, y: f32) -> Self {
        Self(Acceleration2d::new(x, y))
    }

    pub const fn zero() -> Self {
        Self(Acceleration2d::zero())
    }
}

fn setup(mut commands: Commands, textures: Res<TextureMapper>) {
    let acceleration = AccelerationInitial::new(0., -9.8);
    let velocity = VelocityInitial::new(5., 15.);
    commands.spawn((
        PointObject,
        acceleration,
        *acceleration,
        velocity,
        *velocity,
        Position2d::zero(),
        Sprite {
            image: textures.ball_2d_128_128.clone(),
            ..default()
        },
    ));
}

fn solve(
    object: Single<(&Acceleration2d, &mut Velocity2d, &mut Position2d), With<PointObject>>,
    time: Res<Time>,
) {
    let (acceleration, mut velocity, mut position) = object.into_inner();
    let dt = time.delta_secs();
    let a = acceleration.acceleration;

    position.position += velocity.velocity * dt + 0.5 * a * dt * dt;
    velocity.velocity += a * dt;
}

fn sync_transforms(
    mut objects: Query<(&Position2d, &mut Transform)>,
    pixels_per_meter: Res<PixelsPerMeter>,
) {
    for (position, mut transform) in &mut objects {
        let z = transform.translation.z;
        transform.translation = (position.position * pixels_per_meter.0).extend(z);
    }
}

fn sync_camera(
    object: Single<&Transform, (With<PointObject>, Without<Camera2d>)>,
    camera: Single<&mut Transform, With<Camera2d>>,
) {
    let object_transform = object.into_inner();
    let mut camera_transform = camera.into_inner();
    *camera_transform = *object_transform;
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use bevy::time::TimeUpdateStrategy;

    use super::*;

    #[test]
    fn stepping_matches_the_closed_form() {
        let (v_0, a) = (Vec2::new(3.0, 5.0), Vec2::new(0.0, -9.8));
        let dt = Duration::from_secs_f32(1.0 / 64.0);
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(TimeUpdateStrategy::ManualDuration(dt))
            .add_systems(Update, solve);
        app.world_mut().spawn((
            PointObject,
            Acceleration2d::new(a.x, a.y),
            Velocity2d::new(v_0.x, v_0.y),
            Position2d::zero(),
        ));
        // The first update only starts the clock (zero delta).
        app.update();

        let steps = 128;
        for _ in 0..steps {
            app.update();
        }

        let t = dt.as_secs_f32() * steps as f32;
        let (velocity, position) = app
            .world_mut()
            .query::<(&Velocity2d, &Position2d)>()
            .single(app.world())
            .unwrap();
        let x = position.position;
        let expected_x = v_0 * t + 0.5 * a * t * t;
        let expected_v = v_0 + a * t;
        assert!((x - expected_x).length() < 1e-3, "{x} vs {expected_x}");
        assert!(
            (velocity.velocity - expected_v).length() < 1e-3,
            "{} vs {expected_v}",
            velocity.velocity
        );
    }

    #[test]
    fn transforms_follow_position_in_pixels() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(PixelsPerMeter(50.0))
            .add_systems(Update, sync_transforms);
        let object = app
            .world_mut()
            .spawn((
                Position2d::new(2.0, -1.0),
                Transform::from_xyz(0.0, 0.0, 3.0),
            ))
            .id();
        app.update();
        assert_eq!(
            app.world().get::<Transform>(object).unwrap().translation,
            Vec3::new(100.0, -50.0, 3.0),
            "scaled, with z kept"
        );

        app.insert_resource(PixelsPerMeter(10.0));
        app.update();
        assert_eq!(
            app.world().get::<Transform>(object).unwrap().translation,
            Vec3::new(20.0, -10.0, 3.0),
            "rescales immediately"
        );
    }
}
