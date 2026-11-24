use bevy::{
    camera::CameraUpdateSystems,
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
    sprite_render::{AlphaMode2d, Material2d, Material2dPlugin},
};

const SHADER_ASSET_PATH: &str = "shaders/grid.wesl";

/// Draws a world-anchored grid behind everything, covering the camera's whole view.
pub(super) struct GridPlugin;

impl Plugin for GridPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Material2dPlugin::<GridMaterial>::default())
            .add_systems(Startup, setup)
            .add_systems(
                PostUpdate,
                follow_camera
                    .after(CameraUpdateSystems)
                    .before(TransformSystems::Propagate),
            );
    }
}

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct GridMaterial {
    #[uniform(0)]
    pub uniform: GridUniform,
}

#[derive(ShaderType, Debug, Clone, Copy)]
pub struct GridUniform {
    pub line_color: LinearRgba,
    /// World units between gridlines on each axis.
    pub spacing: Vec2,
    /// Line thickness in screen pixels.
    pub line_width: f32,
}

impl Default for GridUniform {
    fn default() -> Self {
        Self {
            line_color: LinearRgba::new(1.0, 1.0, 1.0, 0.15),
            spacing: Vec2::splat(100.0),
            line_width: 1.0,
        }
    }
}

impl Material2d for GridMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER_ASSET_PATH.into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

#[derive(Component, Debug, Copy, Clone)]
pub struct Grid;

/// World-space z of the grid quad, behind sprites at z = 0.
const GRID_Z: f32 = -100.0;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<GridMaterial>>,
) {
    commands.spawn((
        Grid,
        Mesh2d(meshes.add(Rectangle::default())),
        MeshMaterial2d(materials.add(GridMaterial {
            uniform: GridUniform::default(),
        })),
        Transform::from_xyz(0.0, 0.0, GRID_Z),
    ));
}

/// Keeps the grid quad covering the camera's view. The lines themselves are computed from world
/// position in the shader, so moving the quad doesn't move them.
fn follow_camera(
    camera: Query<(&Transform, &Projection), (With<Camera2d>, Without<Grid>)>,
    mut grid: Query<&mut Transform, With<Grid>>,
) -> Result {
    let (camera_transform, projection) = camera.single()?;
    let Projection::Orthographic(ortho) = projection else {
        return Ok(());
    };
    let mut grid_transform = grid.single_mut()?;

    // `area` is in view space; map its center and size into world space. Ignores camera rotation.
    let scale = camera_transform.scale.xy();
    let center = camera_transform.translation.xy() + ortho.area.center() * scale;
    let size = ortho.area.size() * scale;

    grid_transform.translation = center.extend(GRID_Z);
    grid_transform.scale = size.extend(1.0);
    Ok(())
}
