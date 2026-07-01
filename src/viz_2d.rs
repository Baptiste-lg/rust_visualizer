// src/viz_2d.rs

use crate::{audio::AudioAnalysis, config::VisualsConfig, AppState, VisualizationEnabled};
use bevy::prelude::*;

pub struct Viz2DPlugin;

// A resource to track the state of the bar chart, specifically the number of bands.
// This helps in detecting when the chart needs to be rebuilt.
#[derive(Resource, Default)]
struct BarChartState {
    num_bands: usize,
}

// A marker component for the root entity of the 2D visualization scene.
#[derive(Component)]
struct Viz2DScene;

impl Plugin for Viz2DPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BarChartState>()
            .add_systems(OnEnter(AppState::Visualization2D), setup_2d_scene)
            .add_systems(
                Update,
                (manage_bar_chart, update_2d_visuals.after(manage_bar_chart))
                    .run_if(in_state(AppState::Visualization2D))
                    .run_if(|viz_enabled: Res<VisualizationEnabled>| viz_enabled.0),
            )
            .add_systems(OnExit(AppState::Visualization2D), despawn_scene);
    }
}

// A component attached to each bar in the 2D visualizer.
// It stores the index of the frequency band this bar represents.
#[derive(Component)]
struct VizBar {
    index: usize,
}

// Sets up the initial scene for the 2D visualizer by spawning a root entity.
fn setup_2d_scene(mut commands: Commands) {
    commands.spawn((SpatialBundle::default(), Viz2DScene));
}

// Despawns the entire 2D visualizer scene and resets its state
// when exiting the `Visualization2D` state.
fn despawn_scene(mut commands: Commands, scene_query: Query<Entity, With<Viz2DScene>>) {
    if let Ok(entity) = scene_query.get_single() {
        commands.entity(entity).despawn_recursive();
    }
    commands.insert_resource(BarChartState::default());
}

// Manages the bar chart by checking if the number of bands has changed.
// Only rebuilds the bars when necessary, not on every config tweak.
fn manage_bar_chart(
    mut commands: Commands,
    config: Res<VisualsConfig>,
    mut chart_state: ResMut<BarChartState>,
    bar_query: Query<Entity, With<VizBar>>,
    scene_query: Query<Entity, With<Viz2DScene>>,
) {
    if config.num_bands != chart_state.num_bands {
        if let Ok(scene_entity) = scene_query.get_single() {
            for entity in &bar_query {
                commands.entity(entity).despawn_recursive();
            }
            spawn_visuals(commands, &config, scene_entity);
            chart_state.num_bands = config.num_bands;
        }
    }
}

// Spawns the individual bars for the 2D visualizer.
fn spawn_visuals(mut commands: Commands, config: &VisualsConfig, parent_entity: Entity) {
    let num_bars = config.num_bands;
    if num_bars == 0 {
        return;
    }
    let bar_width = 40.0;
    let spacing = 10.0;
    let total_width =
        (num_bars as f32 * bar_width) + ((num_bars.saturating_sub(1)) as f32 * spacing);
    let start_x = -total_width / 2.0;

    commands.entity(parent_entity).with_children(|parent| {
        for i in 0..num_bars {
            let x_pos = start_x + (i as f32 * (bar_width + spacing)) + bar_width / 2.0;
            parent.spawn((
                SpriteBundle {
                    sprite: Sprite {
                        color: config.viz2d_inactive_color,
                        custom_size: Some(Vec2::new(bar_width, 50.0)),
                        ..default()
                    },
                    transform: Transform::from_translation(Vec3::new(x_pos, 0.0, 0.0)),
                    ..default()
                },
                VizBar { index: i },
            ));
        }
    });
}

// Updates the height and color of the bars based on the audio analysis.
fn update_2d_visuals(
    time: Res<Time>,
    audio_analysis: Res<AudioAnalysis>,
    config: Res<VisualsConfig>,
    mut query: Query<(&mut Sprite, &mut Transform, &VizBar)>,
) {
    if audio_analysis.frequency_bins.len() != config.num_bands {
        return;
    }

    let smoothing_speed = 8.0;
    let smoothing_factor = 1.0 - (-smoothing_speed * time.delta_seconds()).exp();

    for (mut sprite, mut transform, bar) in &mut query {
        if let Some(amplitude) = audio_analysis.frequency_bins.get(bar.index) {
            let target_height = 50.0 + amplitude * config.bass_sensitivity * 100.0;

            // Apply smoothing to the height change for a smoother animation.
            let current_size = sprite.custom_size.unwrap_or(Vec2::ZERO);
            let new_height = current_size.y + (target_height - current_size.y) * smoothing_factor;
            sprite.custom_size = Some(Vec2::new(current_size.x, new_height));

            // Adjust the y-position to keep the base of the bar aligned.
            transform.translation.y = new_height / 2.0 - 25.0;

            // Interpolate the bar's color based on its height.
            let color_intensity = (new_height / 800.0).clamp(0.0, 1.0);
            let inactive = config.viz2d_inactive_color;
            let active = config.viz2d_active_color;

            let r = inactive.r() + (active.r() - inactive.r()) * color_intensity;
            let g = inactive.g() + (active.g() - inactive.g()) * color_intensity;
            let b = inactive.b() + (active.b() - inactive.b()) * color_intensity;
            let a = inactive.a() + (active.a() - inactive.a()) * color_intensity;

            sprite.color = Color::rgba(r, g, b, a);
        }
    }
}
