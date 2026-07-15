// src/viz_matrix.rs

use crate::{audio::AudioAnalysis, config::VisualsConfig, AppState, VisualizationEnabled};
use bevy::prelude::*;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};

pub struct VizMatrixPlugin;

#[derive(Component)]
struct MatrixScene;

#[derive(Component)]
struct MatrixColumn {
    x: f32,
    chars: Vec<MatrixChar>,
}

struct MatrixChar {
    y: f32,
    char_idx: usize,
    brightness: f32,
    speed: f32,
}

const MATRIX_CHARS: &str =
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789@#$%&*+=<>?/|\\{}[]~^";

impl Plugin for VizMatrixPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::VisualizationMatrix), setup_matrix)
            .add_systems(
                Update,
                (update_matrix, draw_matrix)
                    .chain()
                    .run_if(in_state(AppState::VisualizationMatrix))
                    .run_if(|viz_enabled: Res<VisualizationEnabled>| viz_enabled.0),
            )
            .add_systems(OnExit(AppState::VisualizationMatrix), despawn_matrix);
    }
}

fn setup_matrix(mut commands: Commands, config: Res<VisualsConfig>) {
    commands.spawn((SpatialBundle::default(), MatrixScene));

    let col_count = config.matrix_columns.max(1);
    let spacing = 1200.0 / col_count as f32;

    for i in 0..col_count {
        let x = -600.0 + (i as f32 + 0.5) * spacing;
        let trail_len = 8 + (rand_u64() % 15) as usize;
        let base_speed = 100.0 + (rand_u64() % 150) as f32;

        let mut chars = Vec::with_capacity(trail_len);
        let start_y = 400.0 + (rand_u64() % 400) as f32;
        for j in 0..trail_len {
            chars.push(MatrixChar {
                y: start_y + j as f32 * 20.0,
                char_idx: (rand_u64() % MATRIX_CHARS.len() as u64) as usize,
                brightness: 1.0 - (j as f32 / trail_len as f32),
                speed: base_speed,
            });
        }

        commands.spawn((MatrixColumn { x, chars }, MatrixScene));
    }
}

fn despawn_matrix(mut commands: Commands, query: Query<Entity, With<MatrixScene>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

fn update_matrix(
    time: Res<Time>,
    audio: Res<AudioAnalysis>,
    config: Res<VisualsConfig>,
    mut columns: Query<&mut MatrixColumn>,
) {
    let dt = time.delta_seconds();
    let volume_boost = 1.0 + audio.volume * config.bass_sensitivity * 5.0;
    let density_threshold = 1.0 - config.matrix_density;

    for mut col in columns.iter_mut() {
        for ch in col.chars.iter_mut() {
            ch.y -= config.matrix_speed * ch.speed / 100.0 * volume_boost * dt;

            if ch.y < -500.0 {
                ch.y = 400.0 + (rand_u64() % 200) as f32;
                ch.char_idx = (rand_u64() % MATRIX_CHARS.len() as u64) as usize;

                if (rand_u64() % 100) as f32 / 100.0 > density_threshold {
                    ch.brightness = 1.0;
                } else {
                    ch.brightness = 0.0;
                }
            }

            if rand_u64() % 20 == 0 {
                ch.char_idx = (rand_u64() % MATRIX_CHARS.len() as u64) as usize;
            }
        }
    }
}

fn draw_matrix(mut gizmos: Gizmos, config: Res<VisualsConfig>, columns: Query<&MatrixColumn>) {
    let base_r = config.matrix_color.r();
    let base_g = config.matrix_color.g();
    let base_b = config.matrix_color.b();

    for col in columns.iter() {
        for (i, ch) in col.chars.iter().enumerate() {
            if ch.brightness <= 0.0 {
                continue;
            }

            let b = ch.brightness;
            let color = if i == 0 {
                Color::rgb(
                    (base_r + 0.7).min(1.0),
                    (base_g + 0.7).min(1.0),
                    (base_b + 0.7).min(1.0),
                )
            } else {
                Color::rgba(base_r * b, base_g * b, base_b * b, b)
            };

            let pos = Vec2::new(col.x, ch.y);
            gizmos.rect_2d(pos, 0.0, Vec2::new(8.0, 14.0), color);
        }
    }
}

fn rand_u64() -> u64 {
    static COUNTER: AtomicU64 = AtomicU64::new(12345);
    let val = COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut hasher = DefaultHasher::new();
    val.hash(&mut hasher);
    hasher.finish()
}
