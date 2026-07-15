// src/viz_kaleidoscope.rs
//
// Shader-based kaleidoscope visualization. The fragment shader folds UV space
// into angular segments, then renders layered organic patterns (flow fields,
// voronoi cells, radial waves) that respond to audio frequencies.

use crate::{audio::AudioAnalysis, camera::MainCamera2D, config::VisualsConfig, AppState};
use bevy::{
    prelude::*,
    reflect::TypePath,
    render::render_resource::{AsBindGroup, ShaderRef},
    sprite::{Material2d, Material2dPlugin, MaterialMesh2dBundle},
    window::PrimaryWindow,
};

pub struct VizKaleidoscopePlugin;

impl Plugin for VizKaleidoscopePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Material2dPlugin::<KaleidoscopeMaterial>::default())
            .add_systems(
                OnEnter(AppState::VisualizationKaleidoscope),
                setup_kaleidoscope,
            )
            .add_systems(
                Update,
                update_kaleidoscope.run_if(in_state(AppState::VisualizationKaleidoscope)),
            )
            .add_systems(
                OnExit(AppState::VisualizationKaleidoscope),
                despawn_kaleidoscope,
            );
    }
}

#[derive(Component)]
struct KaleidoscopeScene;

#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
#[repr(C)]
pub struct KaleidoscopeMaterial {
    #[uniform(0)]
    color: Vec4,
    #[uniform(0)]
    time: f32,
    #[uniform(0)]
    speed: f32,
    #[uniform(0)]
    segments: f32,
    #[uniform(0)]
    pattern_zoom: f32,
    #[uniform(0)]
    resolution: Vec2,
    #[uniform(0)]
    bass: f32,
    #[uniform(0)]
    mid: f32,
    #[uniform(0)]
    treble: f32,
    #[uniform(0)]
    flux: f32,
    #[uniform(0)]
    zoom: f32,
    #[uniform(0)]
    _pad: f32,
}

impl Material2d for KaleidoscopeMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/kaleidoscope_shader.wgsl".into()
    }
}

fn setup_kaleidoscope(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<KaleidoscopeMaterial>>,
    config: Res<VisualsConfig>,
) {
    let quad = meshes.add(Rectangle::new(1.0, 1.0));
    let mat = materials.add(KaleidoscopeMaterial {
        color: Vec4::from(config.kaleidoscope_color.as_linear_rgba_f32()),
        time: 0.0,
        speed: config.kaleidoscope_speed,
        segments: config.kaleidoscope_segments,
        pattern_zoom: config.kaleidoscope_zoom,
        resolution: Vec2::new(800.0, 600.0),
        bass: 0.0,
        mid: 0.0,
        treble: 0.0,
        flux: 0.0,
        zoom: 1.0,
        _pad: 0.0,
    });

    commands.spawn((
        MaterialMesh2dBundle {
            mesh: quad.into(),
            material: mat,
            transform: Transform::from_scale(Vec3::splat(1_000_000.0)),
            ..default()
        },
        KaleidoscopeScene,
    ));
}

fn update_kaleidoscope(
    time: Res<Time>,
    config: Res<VisualsConfig>,
    audio: Res<AudioAnalysis>,
    mut materials: ResMut<Assets<KaleidoscopeMaterial>>,
    q_window: Query<&Window, With<PrimaryWindow>>,
    q_camera: Query<&OrthographicProjection, With<MainCamera2D>>,
    q_scene: Query<&Handle<KaleidoscopeMaterial>, With<KaleidoscopeScene>>,
) {
    let Ok(window) = q_window.get_single() else {
        return;
    };

    let resolution = Vec2::new(
        window.resolution.physical_width() as f32,
        window.resolution.physical_height() as f32,
    );

    let zoom = q_camera
        .get_single()
        .map(|p| p.scale)
        .unwrap_or(1.0);

    let sens = config.bass_sensitivity;

    for handle in &q_scene {
        if let Some(mat) = materials.get_mut(handle) {
            mat.color = Vec4::from(config.kaleidoscope_color.as_linear_rgba_f32());
            mat.time = time.elapsed_seconds();
            mat.speed = config.kaleidoscope_speed;
            mat.segments = config.kaleidoscope_segments;
            mat.pattern_zoom = config.kaleidoscope_zoom;
            mat.resolution = resolution;
            mat.bass = audio.bass * sens;
            mat.mid = audio.mid * sens;
            mat.treble = audio.treble * sens;
            mat.flux = audio.flux * sens;
            mat.zoom = zoom;
        }
    }
}

fn despawn_kaleidoscope(mut commands: Commands, query: Query<Entity, With<KaleidoscopeScene>>) {
    if let Ok(entity) = query.get_single() {
        commands.entity(entity).despawn_recursive();
    }
}
