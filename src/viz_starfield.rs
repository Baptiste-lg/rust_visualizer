// src/viz_starfield.rs

use crate::{audio::AudioAnalysis, config::VisualsConfig, AppState, VisualizationEnabled};
use bevy::prelude::*;

pub struct VizStarfieldPlugin;

#[derive(Component)]
struct StarfieldScene;

#[derive(Component)]
struct Star {
    pos: Vec3,
    base_speed: f32,
}

impl Plugin for VizStarfieldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::VisualizationStarfield), setup_starfield)
            .add_systems(
                Update,
                update_starfield
                    .run_if(in_state(AppState::VisualizationStarfield))
                    .run_if(|viz_enabled: Res<VisualizationEnabled>| viz_enabled.0),
            )
            .add_systems(OnExit(AppState::VisualizationStarfield), despawn_starfield);
    }
}

fn setup_starfield(mut commands: Commands, config: Res<VisualsConfig>) {
    commands.spawn((SpatialBundle::default(), StarfieldScene));

    let spread = config.starfield_spread;
    for _ in 0..config.starfield_count {
        let x = (rand_f32() - 0.5) * spread * 2.0;
        let y = (rand_f32() - 0.5) * spread * 2.0;
        let z = rand_f32() * spread;

        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: config.starfield_color,
                    custom_size: Some(Vec2::splat(2.0)),
                    ..default()
                },
                transform: Transform::from_xyz(x, y, 0.0),
                ..default()
            },
            Star {
                pos: Vec3::new(x, y, z),
                base_speed: 0.5 + rand_f32() * 1.5,
            },
        ));
    }
}

fn despawn_starfield(
    mut commands: Commands,
    scene_query: Query<Entity, With<StarfieldScene>>,
    star_query: Query<Entity, With<Star>>,
) {
    if let Ok(entity) = scene_query.get_single() {
        commands.entity(entity).despawn_recursive();
    }
    for entity in star_query.iter() {
        commands.entity(entity).despawn();
    }
}

fn update_starfield(
    time: Res<Time>,
    audio: Res<AudioAnalysis>,
    config: Res<VisualsConfig>,
    mut query: Query<(&mut Transform, &mut Star, &mut Sprite)>,
) {
    let dt = time.delta_seconds();
    let bass_boost = 1.0 + audio.bass * config.bass_sensitivity * 3.0;
    let spread = config.starfield_spread;

    for (mut transform, mut star, mut sprite) in query.iter_mut() {
        star.pos.z -= config.starfield_speed * star.base_speed * bass_boost * dt;

        if star.pos.z <= 0.1 {
            star.pos.x = (rand_f32() - 0.5) * spread * 2.0;
            star.pos.y = (rand_f32() - 0.5) * spread * 2.0;
            star.pos.z = spread;
        }

        let perspective = 300.0 / star.pos.z;
        transform.translation.x = star.pos.x * perspective;
        transform.translation.y = star.pos.y * perspective;

        let size = (3.0 * perspective).clamp(0.5, 6.0);
        sprite.custom_size = Some(Vec2::splat(size));

        let brightness = (1.0 - star.pos.z / spread).clamp(0.0, 1.0);
        let t = (audio.treble * 2.0).clamp(0.0, 1.0);
        sprite.color = Color::rgb(
            config.starfield_color.r() * brightness + t * 0.3,
            config.starfield_color.g() * brightness * (1.0 - t * 0.3),
            config.starfield_color.b() * brightness,
        );
    }
}

fn rand_f32() -> f32 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let val = COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut hasher = DefaultHasher::new();
    val.hash(&mut hasher);
    (hasher.finish() % 10000) as f32 / 10000.0
}
