mod settings;

use bevy::prelude::*;

pub use self::settings::Kinematics2dSettings;
use crate::{
    Acceleration2d, TextureMapper, Velocity2d, screens::Screen, settings::RegisterSettings,
};

pub(super) struct KinematicsPlugin;

impl Plugin for KinematicsPlugin {
    fn build(&self, app: &mut App) {
        app.register_settings::<Kinematics2dSettings>()
            .add_sub_state::<SimulationState>()
            .add_systems(OnEnter(Screen::Kinematics2d), setup)
            .add_systems(
                FixedUpdate,
                solve.run_if(in_state(SimulationState::Running)),
            );
    }
}

#[derive(SubStates, Copy, Clone, Eq, PartialEq, Hash, Debug, Default)]
#[source(Screen = Screen::Kinematics2d)]
enum SimulationState {
    Running,
    #[default]
    Paused,
}

#[derive(Component, Debug, Copy, Clone)]
pub struct PointObject;

// Newtypes, not aliases: an alias is the same component type as the one it names, and an
// entity can hold only one component of each type.
#[derive(Component, Debug, Copy, Clone, Deref, DerefMut)]
pub struct VelocityInitial(pub Velocity2d);

impl VelocityInitial {
    pub const fn zero() -> Self {
        Self(Velocity2d::zero())
    }
}

#[derive(Component, Debug, Copy, Clone, Deref, DerefMut)]
pub struct AccelerationInitial(pub Acceleration2d);

impl AccelerationInitial {
    pub const fn zero() -> Self {
        Self(Acceleration2d::zero())
    }
}

fn setup(mut commands: Commands, textures: Res<TextureMapper>) {
    commands.spawn((
        PointObject,
        AccelerationInitial::zero(),
        Acceleration2d::zero(),
        VelocityInitial::zero(),
        Velocity2d::zero(),
        Transform::from_xyz(0., 0., 0.),
        Sprite {
            image: textures.ball_2d_128_128.clone(),
            ..default()
        },
    ));
}

fn solve(
    mut object: Query<
        (
            &AccelerationInitial,
            &mut Acceleration2d,
            &VelocityInitial,
            &mut Velocity2d,
            &mut Transform,
        ),
        With<PointObject>,
    >,
    time: Res<Time>,
) -> Result {
    let (
        acceleration_initial,
        acceleration_current,
        velocity_initial,
        mut velocity_current,
        mut transform,
    ) = object.single_mut()?;

    let t = time.delta().as_secs_f32();

    velocity_current.velocity = velocity_initial.velocity + acceleration_initial.acceleration * t;
    let xy = transform.translation.xy()
        + velocity_current.velocity * t
        + 0.5 * acceleration_current.acceleration * t * t;
    transform.translation = xy.extend(transform.translation.z);
    Ok(())
}
