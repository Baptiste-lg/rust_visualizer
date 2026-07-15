// src/viz_tunnel.rs
//
// Shader-based infinite tunnel visualization. The tunnel depth is created
// by inverse-distance mapping, with rings and angular segments that react
// to audio frequencies.

use crate::{audio::AudioAnalysis, camera::MainCamera2D, config::VisualsConfig, AppState};
use bevy::{
    prelude::*,
    reflect::TypePath,
    render::render_resource::{AsBindGroup, ShaderRef},
    sprite::{Material2d, Material2dPlugin, MaterialMesh2dBundle},
    window::PrimaryWindow,
};

pub struct VizTunnelPlugin;

impl Plugin for VizTunnelPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Material2dPlugin::<TunnelMaterial>::default())
            .add_systems(OnEnter(AppState::VisualizationTunnel), setup_tunnel)
            .add_systems(
                Update,
                update_tunnel.run_if(in_state(AppState::VisualizationTunnel)),
            )
            .add_systems(OnExit(AppState::VisualizationTunnel), despawn_tunnel);
    }
}

#[derive(Component)]
struct TunnelScene;

#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
#[repr(C)]
pub struct TunnelMaterial {
    #[uniform(0)]
    color: Vec4,
    #[uniform(0)]
    time: f32,
    #[uniform(0)]
    speed: f32,
    #[uniform(0)]
    ring_count: f32,
    #[uniform(0)]
    twist: f32,
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

impl Material2d for TunnelMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/tunnel_shader.wgsl".into()
    }
}

fn setup_tunnel(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<TunnelMaterial>>,
    config: Res<VisualsConfig>,
) {
    let quad = meshes.add(Rectangle::new(1.0, 1.0));
    let mat = materials.add(TunnelMaterial {
        color: Vec4::from(config.tunnel_color.as_linear_rgba_f32()),
        time: 0.0,
        speed: config.tunnel_speed,
        ring_count: config.tunnel_ring_count,
        twist: config.tunnel_twist,
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
        TunnelScene,
    ));
}

fn update_tunnel(
    time: Res<Time>,
    config: Res<VisualsConfig>,
    audio: Res<AudioAnalysis>,
    mut materials: ResMut<Assets<TunnelMaterial>>,
    q_window: Query<&Window, With<PrimaryWindow>>,
    q_camera: Query<&OrthographicProjection, With<MainCamera2D>>,
    q_tunnel: Query<&Handle<TunnelMaterial>, With<TunnelScene>>,
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

    for handle in &q_tunnel {
        if let Some(mat) = materials.get_mut(handle) {
            mat.color = Vec4::from(config.tunnel_color.as_linear_rgba_f32());
            mat.time = time.elapsed_seconds();
            mat.speed = config.tunnel_speed;
            mat.ring_count = config.tunnel_ring_count;
            mat.twist = config.tunnel_twist;
            mat.resolution = resolution;
            mat.bass = audio.bass * config.bass_sensitivity;
            mat.mid = audio.mid * config.bass_sensitivity;
            mat.treble = audio.treble * config.bass_sensitivity;
            mat.flux = audio.flux;
            mat.zoom = zoom;
        }
    }
}

fn despawn_tunnel(mut commands: Commands, query: Query<Entity, With<TunnelScene>>) {
    if let Ok(entity) = query.get_single() {
        commands.entity(entity).despawn_recursive();
    }
}
