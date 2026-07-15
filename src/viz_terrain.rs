// src/viz_terrain.rs
//
// A 3D terrain grid whose vertex heights are driven by frequency bands.
// Each row of vertices maps to a frequency band, creating a landscape
// that ripples and pulses with the music.

use crate::{audio::AudioAnalysis, config::VisualsConfig, AppState, VisualizationEnabled};
use bevy::{
    prelude::*,
    render::{
        mesh::{Indices, Mesh, PrimitiveTopology, VertexAttributeValues},
        render_asset::RenderAssetUsages,
    },
};

pub struct VizTerrainPlugin;

#[derive(Component)]
struct TerrainScene;

#[derive(Component)]
struct TerrainMesh {
    grid_size: usize,
}

#[derive(Resource, Default)]
struct TerrainState {
    last_grid_size: usize,
    vertex_buffer: Vec<[f32; 3]>,
    normal_buffer: Vec<[f32; 3]>,
}

impl Plugin for VizTerrainPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TerrainState>()
            .add_systems(OnEnter(AppState::VisualizationTerrain), setup_terrain)
            .add_systems(
                Update,
                (manage_terrain_grid, update_terrain)
                    .chain()
                    .run_if(in_state(AppState::VisualizationTerrain))
                    .run_if(|viz_enabled: Res<VisualizationEnabled>| viz_enabled.0),
            )
            .add_systems(OnExit(AppState::VisualizationTerrain), despawn_terrain);
    }
}

fn build_terrain_mesh(grid_size: usize) -> Mesh {
    let mut positions: Vec<[f32; 3]> = Vec::new();
    let mut normals: Vec<[f32; 3]> = Vec::new();
    let mut uvs: Vec<[f32; 2]> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    let size = grid_size as f32;
    let half = size / 2.0;

    for z in 0..=grid_size {
        for x in 0..=grid_size {
            let px = (x as f32 - half) * 0.5;
            let pz = (z as f32 - half) * 0.5;
            positions.push([px, 0.0, pz]);
            normals.push([0.0, 1.0, 0.0]);
            uvs.push([x as f32 / size, z as f32 / size]);
        }
    }

    let row_len = (grid_size + 1) as u32;
    for z in 0..grid_size as u32 {
        for x in 0..grid_size as u32 {
            let tl = z * row_len + x;
            let tr = tl + 1;
            let bl = (z + 1) * row_len + x;
            let br = bl + 1;
            indices.extend_from_slice(&[tl, bl, tr, tr, bl, br]);
        }
    }

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

fn setup_terrain(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    config: Res<VisualsConfig>,
    mut state: ResMut<TerrainState>,
) {
    let grid_size = config.terrain_grid_size.clamp(8, 128);
    state.last_grid_size = grid_size;

    let mesh = build_terrain_mesh(grid_size);

    commands.spawn((
        PbrBundle {
            mesh: meshes.add(mesh),
            material: materials.add(StandardMaterial {
                base_color: config.terrain_low_color,
                perceptual_roughness: 0.6,
                metallic: 0.3,
                double_sided: true,
                cull_mode: None,
                ..default()
            }),
            transform: Transform::from_xyz(0.0, -2.0, 0.0),
            ..default()
        },
        TerrainMesh { grid_size },
        TerrainScene,
    ));
}

fn manage_terrain_grid(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    config: Res<VisualsConfig>,
    mut state: ResMut<TerrainState>,
    query: Query<Entity, With<TerrainMesh>>,
) {
    let grid_size = config.terrain_grid_size.clamp(8, 128);
    if grid_size == state.last_grid_size {
        return;
    }

    for entity in query.iter() {
        commands.entity(entity).despawn();
    }

    state.last_grid_size = grid_size;
    let mesh = build_terrain_mesh(grid_size);

    commands.spawn((
        PbrBundle {
            mesh: meshes.add(mesh),
            material: materials.add(StandardMaterial {
                base_color: config.terrain_low_color,
                perceptual_roughness: 0.6,
                metallic: 0.3,
                double_sided: true,
                cull_mode: None,
                ..default()
            }),
            transform: Transform::from_xyz(0.0, -2.0, 0.0),
            ..default()
        },
        TerrainMesh { grid_size },
        TerrainScene,
    ));
}

fn update_terrain(
    time: Res<Time>,
    config: Res<VisualsConfig>,
    audio: Res<AudioAnalysis>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut state: ResMut<TerrainState>,
    query: Query<(&Handle<Mesh>, &Handle<StandardMaterial>, &TerrainMesh)>,
) {
    if audio.frequency_bins.is_empty() {
        return;
    }

    let t = time.elapsed_seconds() * config.terrain_wave_speed;

    for (mesh_handle, mat_handle, terrain) in query.iter() {
        let grid = terrain.grid_size;
        let row_len = grid + 1;
        let vert_count = row_len * row_len;

        // Ensure cached buffers are the right size (no alloc if already sized)
        state.vertex_buffer.resize(vert_count, [0.0; 3]);
        state.normal_buffer.resize(vert_count, [0.0, 1.0, 0.0]);

        if let Some(mesh) = meshes.get_mut(mesh_handle) {
            let Some(VertexAttributeValues::Float32x3(existing)) =
                mesh.attribute(Mesh::ATTRIBUTE_POSITION)
            else {
                continue;
            };

            // Copy base positions into cached buffer (only x/z, we overwrite y)
            state.vertex_buffer.copy_from_slice(existing);

            let bin_count = audio.frequency_bins.len();

            for z in 0..=grid {
                let bin_idx = ((z as f32 / grid as f32) * (bin_count - 1) as f32) as usize;
                let amplitude =
                    audio.frequency_bins[bin_idx.min(bin_count - 1)] * config.bass_sensitivity;

                for x in 0..=grid {
                    let idx = z * row_len + x;
                    let xf = x as f32 / grid as f32;
                    let zf = z as f32 / grid as f32;

                    let wave = (xf * 6.0 + t).sin() * 0.3 + (zf * 4.0 + t * 0.7).cos() * 0.2;
                    state.vertex_buffer[idx][1] = (amplitude + wave) * config.terrain_height_scale;
                }
            }

            let state = &mut *state;
            compute_normals_into(&state.vertex_buffer, grid, &mut state.normal_buffer);
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, state.vertex_buffer.clone());
            mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, state.normal_buffer.clone());
        }

        if let Some(mat) = materials.get_mut(mat_handle) {
            let intensity = (audio.bass * config.bass_sensitivity).clamp(0.0, 1.0);
            mat.base_color = Color::rgb(
                config.terrain_low_color.r()
                    + (config.terrain_high_color.r() - config.terrain_low_color.r()) * intensity,
                config.terrain_low_color.g()
                    + (config.terrain_high_color.g() - config.terrain_low_color.g()) * intensity,
                config.terrain_low_color.b()
                    + (config.terrain_high_color.b() - config.terrain_low_color.b()) * intensity,
            );
            mat.emissive = config.terrain_high_color * intensity * 0.5;
        }
    }
}

fn compute_normals_into(vertices: &[[f32; 3]], grid: usize, normals: &mut Vec<[f32; 3]>) {
    let row_len = grid + 1;
    let vert_count = vertices.len();
    normals.resize(vert_count, [0.0; 3]);
    normals.fill([0.0; 3]);

    for z in 0..grid {
        for x in 0..grid {
            let tl = z * row_len + x;
            let tr = tl + 1;
            let bl = (z + 1) * row_len + x;
            let br = bl + 1;

            if br >= vert_count {
                continue;
            }

            let v0 = Vec3::from(vertices[tl]);
            let v1 = Vec3::from(vertices[bl]);
            let v2 = Vec3::from(vertices[tr]);
            let v3 = Vec3::from(vertices[br]);

            let n1 = (v1 - v0).cross(v2 - v0).normalize_or_zero();
            let n2 = (v2 - v3).cross(v1 - v3).normalize_or_zero();

            for &idx in &[tl, bl, tr] {
                normals[idx][0] += n1.x;
                normals[idx][1] += n1.y;
                normals[idx][2] += n1.z;
            }
            for &idx in &[tr, bl, br] {
                normals[idx][0] += n2.x;
                normals[idx][1] += n2.y;
                normals[idx][2] += n2.z;
            }
        }
    }

    for n in normals.iter_mut() {
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        if len > 0.0 {
            n[0] /= len;
            n[1] /= len;
            n[2] /= len;
        } else {
            *n = [0.0, 1.0, 0.0];
        }
    }
}

fn despawn_terrain(mut commands: Commands, query: Query<Entity, With<TerrainScene>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}
