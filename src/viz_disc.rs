use crate::{audio::AudioAnalysis, camera::MainCamera2D, config::VisualsConfig, AppState};
use bevy::{
    prelude::*,
    reflect::TypePath,
    render::render_resource::{AsBindGroup, ShaderRef},
    sprite::{Material2d, Material2dPlugin, MaterialMesh2dBundle},
    window::PrimaryWindow,
};

pub struct VizDiscPlugin;

impl Plugin for VizDiscPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Material2dPlugin::<DiscMaterial>::default())
            .add_systems(OnEnter(AppState::VisualizationDisc), setup_disc_scene)
            .add_systems(
                Update,
                update_disc_material.run_if(in_state(AppState::VisualizationDisc)),
            )
            .add_systems(OnExit(AppState::VisualizationDisc), despawn_scene);
    }
}

#[derive(Component)]
struct DiscScene;

// Added #[repr(C)] to guarantee memory alignment with the shader
#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
#[repr(C)]
pub struct DiscMaterial {
    #[uniform(0)]
    color: Vec4, // 16 bytes (offset 0)
    #[uniform(0)]
    time: f32, // 4 bytes  (offset 16)
    #[uniform(0)]
    radius: f32, // 4 bytes  (offset 20)
    #[uniform(0)]
    line_thickness: f32, // 4 bytes  (offset 24)
    #[uniform(0)]
    iterations: f32, // 4 bytes  (offset 28)
    #[uniform(0)]
    speed: f32, // 4 bytes  (offset 32)
    #[uniform(0)]
    center_radius_factor: f32, // 4 bytes  (offset 36)
    #[uniform(0)]
    resolution: Vec2, // 8 bytes  (offset 40)
    #[uniform(0)]
    bass: f32, // 4 bytes  (offset 48)
    #[uniform(0)]
    flux: f32, // 4 bytes  (offset 52)
    #[uniform(0)]
    zoom: f32, // 4 bytes  (offset 56)
    #[uniform(0)]
    _padding: f32, // 4 bytes  (offset 60 -> 64 total)
}

impl Material2d for DiscMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/disc_shader.wgsl".into()
    }
}

fn color_to_vec4(color: Color) -> Vec4 {
    Vec4::from(color.as_linear_rgba_f32())
}

fn setup_disc_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<DiscMaterial>>,
    config: Res<VisualsConfig>,
) {
    let quad_handle = meshes.add(Rectangle::new(1.0, 1.0));

    // Initialize with default values
    let material_handle = materials.add(DiscMaterial {
        color: color_to_vec4(config.disc_color),
        time: 0.0,
        radius: config.disc_radius,
        line_thickness: config.disc_line_thickness,
        iterations: config.disc_iterations as f32,
        speed: config.disc_speed,
        center_radius_factor: config.disc_center_radius_factor,
        resolution: Vec2::new(800.0, 600.0), // Temporary value, updated in the loop
        bass: 0.0,
        flux: 0.0,
        zoom: 1.0,
        _padding: 0.0,
    });

    commands.spawn((
        MaterialMesh2dBundle {
            mesh: quad_handle.into(),
            material: material_handle,
            // Very large quad to cover the screen
            transform: Transform::from_scale(Vec3::splat(1_000_000.0)),
            ..default()
        },
        DiscScene,
    ));
}

fn update_disc_material(
    time: Res<Time>,
    config: Res<VisualsConfig>,
    audio_analysis: Res<AudioAnalysis>,
    mut materials: ResMut<Assets<DiscMaterial>>,
    q_window: Query<&Window, With<PrimaryWindow>>,
    q_camera: Query<&OrthographicProjection, With<MainCamera2D>>,
    q_disc: Query<&Handle<DiscMaterial>, With<DiscScene>>,
) {
    let Ok(window) = q_window.get_single() else {
        return;
    };

    let window_resolution = Vec2::new(
        window.resolution.physical_width() as f32,
        window.resolution.physical_height() as f32,
    );

    let zoom_level = if let Ok(projection) = q_camera.get_single() {
        projection.scale
    } else {
        1.0
    };

    for handle in &q_disc {
        if let Some(material) = materials.get_mut(handle) {
            material.time = time.elapsed_seconds();
            material.color = color_to_vec4(config.disc_color);
            material.radius = config.disc_radius;
            material.line_thickness = config.disc_line_thickness;
            material.iterations = config.disc_iterations as f32;
            material.speed = config.disc_speed;
            material.center_radius_factor = config.disc_center_radius_factor;
            material.resolution = window_resolution;
            material.bass = audio_analysis.bass;
            material.flux = audio_analysis.flux;
            material.zoom = zoom_level;
        }
    }
}

fn despawn_scene(mut commands: Commands, scene_query: Query<Entity, With<DiscScene>>) {
    if let Ok(entity) = scene_query.get_single() {
        commands.entity(entity).despawn_recursive();
    }
}
