// src/viz_particles.rs

use crate::{audio::AudioAnalysis, config::VisualsConfig, AppState, VisualizationEnabled};
use bevy::prelude::*;

pub struct VizParticlesPlugin;

#[derive(Component)]
struct ParticleScene;

#[derive(Component)]
struct Particle {
    velocity: Vec2,
    lifetime: f32,
    max_lifetime: f32,
}

impl Plugin for VizParticlesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::VisualizationParticles), setup_particles)
            .add_systems(
                Update,
                (spawn_particles, update_particles)
                    .run_if(in_state(AppState::VisualizationParticles))
                    .run_if(|viz_enabled: Res<VisualizationEnabled>| viz_enabled.0),
            )
            .add_systems(OnExit(AppState::VisualizationParticles), despawn_particles);
    }
}

fn setup_particles(mut commands: Commands) {
    commands.spawn((SpatialBundle::default(), ParticleScene));
}

fn despawn_particles(
    mut commands: Commands,
    scene_query: Query<Entity, With<ParticleScene>>,
    particle_query: Query<Entity, With<Particle>>,
) {
    if let Ok(entity) = scene_query.get_single() {
        commands.entity(entity).despawn_recursive();
    }
    for entity in particle_query.iter() {
        commands.entity(entity).despawn();
    }
}

const MAX_PARTICLES: usize = 500;
const SPAWN_COOLDOWN: f32 = 0.1;

fn spawn_particles(
    mut commands: Commands,
    audio: Res<AudioAnalysis>,
    config: Res<VisualsConfig>,
    time: Res<Time>,
    mut cooldown: Local<f32>,
    existing: Query<&Particle>,
) {
    *cooldown = (*cooldown - time.delta_seconds()).max(0.0);

    if !audio.beat_detected || *cooldown > 0.0 {
        return;
    }

    if existing.iter().count() >= MAX_PARTICLES {
        return;
    }

    *cooldown = SPAWN_COOLDOWN;

    let count = config.particles_count;
    let speed_base = (audio.bass * config.bass_sensitivity * 100.0).min(600.0);

    for i in 0..count {
        let angle = (i as f32 / count as f32) * std::f32::consts::TAU + audio.treble * 0.5;
        let speed = speed_base * (0.7 + 0.6 * ((i * 7) as f32 % 1.3));
        let velocity = Vec2::new(angle.cos() * speed, angle.sin() * speed);

        commands.spawn((
            SpriteBundle {
                sprite: Sprite {
                    color: config.particles_color,
                    custom_size: Some(Vec2::splat(config.particles_size)),
                    ..default()
                },
                transform: Transform::from_xyz(0.0, 0.0, 0.0),
                ..default()
            },
            Particle {
                velocity,
                lifetime: config.particles_lifetime,
                max_lifetime: config.particles_lifetime,
            },
        ));
    }
}

fn update_particles(
    mut commands: Commands,
    time: Res<Time>,
    config: Res<VisualsConfig>,
    mut query: Query<(Entity, &mut Transform, &mut Particle, &mut Sprite)>,
) {
    let dt = time.delta_seconds();

    for (entity, mut transform, mut particle, mut sprite) in query.iter_mut() {
        particle.lifetime -= dt;
        if particle.lifetime <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }

        particle.velocity.y += config.particles_gravity * dt;

        transform.translation.x += particle.velocity.x * dt;
        transform.translation.y += particle.velocity.y * dt;

        let alpha = (particle.lifetime / particle.max_lifetime).clamp(0.0, 1.0);
        sprite.color = sprite.color.with_a(alpha);
    }
}
