// src/viz_circular.rs

use crate::{audio::AudioAnalysis, config::VisualsConfig, AppState, VisualizationEnabled};
use bevy::prelude::*;

pub struct VizCircularPlugin;

#[derive(Component)]
struct CircularScene;

impl Plugin for VizCircularPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::VisualizationCircular), setup_circular)
            .add_systems(
                Update,
                draw_circular
                    .run_if(in_state(AppState::VisualizationCircular))
                    .run_if(|viz_enabled: Res<VisualizationEnabled>| viz_enabled.0),
            )
            .add_systems(OnExit(AppState::VisualizationCircular), despawn_circular);
    }
}

fn setup_circular(mut commands: Commands) {
    commands.spawn((SpatialBundle::default(), CircularScene));
}

fn despawn_circular(mut commands: Commands, query: Query<Entity, With<CircularScene>>) {
    if let Ok(entity) = query.get_single() {
        commands.entity(entity).despawn_recursive();
    }
}

fn draw_circular(
    mut gizmos: Gizmos,
    audio: Res<AudioAnalysis>,
    config: Res<VisualsConfig>,
    time: Res<Time>,
) {
    let bar_count = config.circular_bar_count.max(1);
    let radius = config.circular_radius;
    let rotation = time.elapsed_seconds() * config.circular_rotation_speed;

    let bins = &audio.frequency_bins;
    if bins.is_empty() {
        return;
    }

    let step = bins.len() as f32 / bar_count as f32;

    for i in 0..bar_count {
        let angle = (i as f32 / bar_count as f32) * std::f32::consts::TAU + rotation;
        let bin_idx = ((i as f32 * step) as usize).min(bins.len() - 1);
        let amplitude = bins[bin_idx] * config.bass_sensitivity;
        let bar_length = amplitude * 100.0;

        let inner = Vec2::new(angle.cos() * radius, angle.sin() * radius);
        let outer = Vec2::new(
            angle.cos() * (radius + bar_length),
            angle.sin() * (radius + bar_length),
        );

        let t = (amplitude * 2.0).clamp(0.0, 1.0);
        let r = config.circular_color.r() + t * (1.0 - config.circular_color.r());
        let g = config.circular_color.g() * (1.0 - t * 0.5);
        let b = config.circular_color.b() * (1.0 - t);
        let color = Color::rgb(r.min(1.0), g.max(0.0), b.max(0.0));

        gizmos.line_2d(inner, outer, color);
    }
}
