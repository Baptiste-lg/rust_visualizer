// src/camera.rs

use crate::{config::VisualsConfig, AppState};
use bevy::{
    core_pipeline::bloom::BloomSettings,
    input::mouse::{MouseMotion, MouseWheel},
    prelude::*,
    window::PrimaryWindow,
};
use bevy_egui::{EguiContexts, EguiSet};

pub struct CameraPlugin;

#[derive(Component)]
pub struct MainCamera3D;

#[derive(Component)]
pub struct MainCamera2D;

#[derive(Component)]
pub struct PanOrbitController {
    pub focus: Vec3,
    pub radius: f32,
    pub enabled: bool,
}

impl Default for PanOrbitController {
    fn default() -> Self {
        PanOrbitController {
            focus: Vec3::ZERO,
            radius: 15.0,
            enabled: true,
        }
    }
}

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app
            // Systems for the 3D camera
            .add_systems(OnEnter(AppState::Visualization3D), setup_3d_camera)
            .add_systems(OnEnter(AppState::VisualizationOrb), setup_3d_camera)
            .add_systems(OnEnter(AppState::VisualizationTerrain), setup_3d_camera)
            .add_systems(OnExit(AppState::Visualization3D), despawn_3d_camera)
            .add_systems(OnExit(AppState::VisualizationOrb), despawn_3d_camera)
            .add_systems(OnExit(AppState::VisualizationTerrain), despawn_3d_camera)
            .add_systems(
                Update,
                (pan_orbit_camera, update_bloom_settings)
                    .run_if(
                        in_state(AppState::Visualization3D)
                            .or_else(in_state(AppState::VisualizationOrb))
                            .or_else(in_state(AppState::VisualizationTerrain)),
                    )
                    .after(EguiSet::InitContexts),
            )
            // Systems for the 2D camera
            .add_systems(OnEnter(AppState::Visualization2D), setup_2d_camera)
            .add_systems(OnEnter(AppState::VisualizationDisc), setup_2d_camera)
            .add_systems(OnEnter(AppState::VisualizationIco), setup_2d_camera)
            .add_systems(OnEnter(AppState::VisualizationWaveform), setup_2d_camera)
            .add_systems(OnEnter(AppState::VisualizationParticles), setup_2d_camera)
            .add_systems(OnEnter(AppState::VisualizationCircular), setup_2d_camera)
            .add_systems(OnEnter(AppState::VisualizationStarfield), setup_2d_camera)
            .add_systems(OnEnter(AppState::VisualizationMatrix), setup_2d_camera)
            .add_systems(OnEnter(AppState::VisualizationTunnel), setup_2d_camera)
            .add_systems(
                OnEnter(AppState::VisualizationKaleidoscope),
                setup_2d_camera,
            )
            .add_systems(OnExit(AppState::Visualization2D), despawn_2d_camera)
            .add_systems(OnExit(AppState::VisualizationDisc), despawn_2d_camera)
            .add_systems(OnExit(AppState::VisualizationIco), despawn_2d_camera)
            .add_systems(OnExit(AppState::VisualizationWaveform), despawn_2d_camera)
            .add_systems(OnExit(AppState::VisualizationParticles), despawn_2d_camera)
            .add_systems(OnExit(AppState::VisualizationCircular), despawn_2d_camera)
            .add_systems(OnExit(AppState::VisualizationStarfield), despawn_2d_camera)
            .add_systems(OnExit(AppState::VisualizationMatrix), despawn_2d_camera)
            .add_systems(OnExit(AppState::VisualizationTunnel), despawn_2d_camera)
            .add_systems(
                OnExit(AppState::VisualizationKaleidoscope),
                despawn_2d_camera,
            )
            .add_systems(
                Update,
                control_2d_camera
                    .run_if(
                        in_state(AppState::Visualization2D)
                            .or_else(in_state(AppState::VisualizationDisc))
                            .or_else(in_state(AppState::VisualizationIco))
                            .or_else(in_state(AppState::VisualizationWaveform))
                            .or_else(in_state(AppState::VisualizationParticles))
                            .or_else(in_state(AppState::VisualizationCircular))
                            .or_else(in_state(AppState::VisualizationStarfield))
                            .or_else(in_state(AppState::VisualizationMatrix))
                            .or_else(in_state(AppState::VisualizationTunnel))
                            .or_else(in_state(AppState::VisualizationKaleidoscope)),
                    )
                    .after(EguiSet::InitContexts),
            );
    }
}

fn setup_3d_camera(mut commands: Commands) {
    let initial_transform = Transform::from_xyz(0.0, 0.0, 15.0).looking_at(Vec3::ZERO, Vec3::Y);

    commands.spawn((
        Camera3dBundle {
            transform: initial_transform,
            camera: Camera {
                hdr: true,
                ..default()
            },
            ..default()
        },
        BloomSettings::default(),
        PanOrbitController::default(),
        MainCamera3D,
    ));

    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 2000.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });
}

fn despawn_3d_camera(
    mut commands: Commands,
    camera_query: Query<Entity, With<MainCamera3D>>,
    light_query: Query<Entity, With<PointLight>>,
) {
    if let Ok(entity) = camera_query.get_single() {
        commands.entity(entity).despawn_recursive();
    }
    if let Ok(entity) = light_query.get_single() {
        commands.entity(entity).despawn_recursive();
    }
}

fn setup_2d_camera(mut commands: Commands) {
    commands.spawn((Camera2dBundle::default(), MainCamera2D));
}

fn despawn_2d_camera(mut commands: Commands, camera_query: Query<Entity, With<MainCamera2D>>) {
    if let Ok(entity) = camera_query.get_single() {
        commands.entity(entity).despawn_recursive();
    }
}

fn update_bloom_settings(
    config: Res<VisualsConfig>,
    mut camera_query: Query<(Entity, Option<&mut BloomSettings>), With<MainCamera3D>>,
    mut commands: Commands,
) {
    if !config.is_changed() {
        return;
    }
    if let Ok((camera_entity, bloom_settings)) = camera_query.get_single_mut() {
        if config.bloom_enabled {
            match bloom_settings {
                Some(mut settings) => {
                    settings.intensity = config.bloom_intensity;
                    settings.prefilter_settings.threshold = config.bloom_threshold;
                }
                None => {
                    commands.entity(camera_entity).insert(BloomSettings {
                        intensity: config.bloom_intensity,
                        prefilter_settings: bevy::core_pipeline::bloom::BloomPrefilterSettings {
                            threshold: config.bloom_threshold,
                            ..default()
                        },
                        ..default()
                    });
                }
            }
        } else if bloom_settings.is_some() {
            commands.entity(camera_entity).remove::<BloomSettings>();
        }
    }
}

fn control_2d_camera(
    mut ev_scroll: EventReader<MouseWheel>,
    mut camera_query: Query<&mut OrthographicProjection, With<MainCamera2D>>,
    mut contexts: EguiContexts,
) {
    #[allow(clippy::collapsible_if)]
    if let Some(ctx) = contexts.try_ctx_mut() {
        if ctx.is_pointer_over_area() || ctx.wants_pointer_input() {
            ev_scroll.clear();
            return;
        }
    }

    if let Ok(mut projection) = camera_query.get_single_mut() {
        for ev in ev_scroll.read() {
            // Safer logic to prevent projection inverting
            let new_scale = projection.scale - ev.y * 0.1;
            projection.scale = new_scale.max(0.1);
        }
    }
}

fn pan_orbit_camera(
    primary_window: Query<&Window, With<PrimaryWindow>>,
    mut ev_motion: EventReader<MouseMotion>,
    mut ev_scroll: EventReader<MouseWheel>,
    input_mouse: Res<ButtonInput<MouseButton>>,
    mut query: Query<(&mut PanOrbitController, &mut Transform), With<MainCamera3D>>,
    mut contexts: EguiContexts,
) {
    #[allow(clippy::collapsible_if)]
    if let Some(ctx) = contexts.try_ctx_mut() {
        if ctx.is_pointer_over_area() || ctx.wants_pointer_input() {
            ev_motion.clear();
            ev_scroll.clear();
            return;
        }
    }

    let Ok(window) = primary_window.get_single() else {
        return;
    };

    if let Ok((mut pan_orbit, mut transform)) = query.get_single_mut() {
        if !pan_orbit.enabled {
            return;
        }

        if input_mouse.pressed(MouseButton::Left) {
            let mut rotation = Vec2::ZERO;
            for ev in ev_motion.read() {
                rotation += ev.delta;
            }

            if rotation.length_squared() > 0.0 {
                let window_size = Vec2::new(window.width(), window.height());
                if window_size.x == 0.0 || window_size.y == 0.0 {
                    return;
                }
                let delta_x = rotation.x / window_size.x * std::f32::consts::PI * 2.0;
                let delta_y = rotation.y / window_size.y * std::f32::consts::PI;
                let yaw = Quat::from_rotation_y(-delta_x);
                let pitch = Quat::from_rotation_x(-delta_y);
                transform.rotation = yaw * transform.rotation * pitch;
            }
        }

        let mut scroll = 0.0;
        for ev in ev_scroll.read() {
            scroll += ev.y;
        }
        if scroll.abs() > 0.0 {
            pan_orbit.radius = (pan_orbit.radius - scroll * pan_orbit.radius * 0.1).max(5.0);
        }

        let rot_matrix = Mat3::from_quat(transform.rotation);
        transform.translation =
            pan_orbit.focus + rot_matrix.mul_vec3(Vec3::new(0.0, 0.0, pan_orbit.radius));
    }
}
