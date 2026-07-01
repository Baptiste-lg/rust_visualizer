// src/viz_orb.rs

use crate::{audio::AudioAnalysis, config::VisualsConfig, AppState, VisualizationEnabled};
use bevy::{
    prelude::*,
    render::mesh::{Mesh, VertexAttributeValues},
};
use noise::{NoiseFn, Perlin};

pub struct VizOrbPlugin;

// A marker component for all visual elements of the orb scene.
#[derive(Component)]
struct OrbVisual;

// A component to store the state of our deformable orb,
// including its original vertex positions and a noise generator.
#[derive(Component)]
struct DeformableOrb {
    original_vertices: Vec<[f32; 3]>,
    noise: Perlin,
}

impl Plugin for VizOrbPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::VisualizationOrb), setup_orb)
            .add_systems(
                Update,
                deform_orb
                    .run_if(in_state(AppState::VisualizationOrb))
                    .run_if(|viz_enabled: Res<VisualizationEnabled>| viz_enabled.0),
            )
            .add_systems(OnExit(AppState::VisualizationOrb), despawn_orb_visuals);
    }
}

// Sets up the orb scene by creating a sphere mesh and preparing it for deformation.
fn setup_orb(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    config: Res<VisualsConfig>,
) {
    // Create a base IcoSphere mesh with a given subdivision level.
    let mut sphere_mesh = Sphere::new(3.0).mesh().ico(5).unwrap();

    // The mesh must be "un-indexed" or "flattened" so that each triangle
    // has its own unique vertices. This is required for `compute_flat_normals`
    // to work correctly and give the orb its low-poly, faceted look.
    sphere_mesh.duplicate_vertices();
    sphere_mesh.compute_flat_normals();

    // Store the original positions of the vertices from the flattened mesh.
    // These will be used as a base for the deformation calculations.
    let original_vertices = match sphere_mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
        Some(VertexAttributeValues::Float32x3(vertices)) => vertices.clone(),
        _ => Vec::new(),
    };

    // Spawn the orb entity.
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(sphere_mesh), // Add the prepared mesh to assets.
            material: materials.add(StandardMaterial {
                base_color: config.orb_base_color,
                perceptual_roughness: 0.8,
                metallic: 0.2,
                emissive: config.orb_base_color,
                ..default()
            }),
            ..default()
        },
        DeformableOrb {
            original_vertices,
            noise: Perlin::new(1), // Initialize the Perlin noise generator.
        },
        OrbVisual,
    ));
}

// This system deforms the orb's mesh and updates its material properties each frame.
fn deform_orb(
    time: Res<Time>,
    config: Res<VisualsConfig>,
    audio_analysis: Res<AudioAnalysis>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut query: Query<(&Handle<Mesh>, &Handle<StandardMaterial>, &DeformableOrb)>,
) {
    if audio_analysis.frequency_bins.is_empty() {
        return;
    }

    // Calculate the total amplitude of the bass frequencies.
    let bass_count = (config.num_bands / 4).max(1);
    let bass_end = bass_count.min(audio_analysis.frequency_bins.len());
    let total_bass_amplitude = audio_analysis.frequency_bins[..bass_end]
        .iter()
        .sum::<f32>()
        / bass_count as f32;

    for (mesh_handle, material_handle, orb) in &mut query {
        if let Some(mesh) = meshes.get_mut(mesh_handle) {
            let Some(vertices) = mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION) else {
                continue;
            };

            // The main deformation logic happens here.
            if let VertexAttributeValues::Float32x3(vertex_data) = vertices {
                if vertex_data.len() != orb.original_vertices.len() {
                    continue;
                }

                for (i, pos) in vertex_data.iter_mut().enumerate() {
                    let original_pos = Vec3::from_array(orb.original_vertices[i]);
                    let normalized_pos = original_pos.normalize();

                    // Influence the noise with time and treble from the audio.
                    let time_val = time.elapsed_seconds() * config.orb_noise_speed;
                    let treble_factor =
                        1.0 + audio_analysis.treble_average * config.orb_treble_influence;
                    let noise_frequency = config.orb_noise_frequency * treble_factor;

                    // Sample the 3D Perlin noise function.
                    let noise_input = (normalized_pos * noise_frequency) + time_val;
                    let noise_value = orb.noise.get([
                        noise_input.x as f64,
                        noise_input.y as f64,
                        noise_input.z as f64,
                    ]) as f32;

                    // Displace the vertex along its normal based on the noise value and bass amplitude.
                    let displacement = noise_value * total_bass_amplitude * config.bass_sensitivity;
                    let new_pos = original_pos + normalized_pos * displacement;

                    // Assign the new position
                    *pos = new_pos.into();
                }
            }

            // Recalculate the flat normals after deforming the vertices.
            // This is safe now because we duplicated the vertices at setup.
            mesh.compute_flat_normals();
        }

        // Update the material's emissive color based on the bass amplitude.
        if let Some(material) = materials.get_mut(material_handle) {
            let emissive_intensity = (total_bass_amplitude * 2.0).clamp(0.0, 5.0);
            material.emissive = config.orb_peak_color * emissive_intensity;
        }
    }
}

// Despawns the orb visuals when exiting the `VisualizationOrb` state.
fn despawn_orb_visuals(mut commands: Commands, query: Query<Entity, With<OrbVisual>>) {
    for entity in &query {
        commands.entity(entity).despawn_recursive();
    }
}
