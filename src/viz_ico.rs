use crate::{audio::AudioAnalysis, camera::MainCamera2D, config::VisualsConfig, AppState};
use bevy::{
    prelude::*,
    reflect::TypePath,
    render::render_resource::{AsBindGroup, ShaderRef},
    sprite::{Material2d, Material2dPlugin, MaterialMesh2dBundle},
    window::PrimaryWindow,
};

pub struct VizIcoPlugin;

impl Plugin for VizIcoPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Material2dPlugin::<IcoMaterial>::default())
            .add_systems(OnEnter(AppState::VisualizationIco), setup_ico_scene)
            .add_systems(
                Update,
                update_ico_material.run_if(in_state(AppState::VisualizationIco)),
            )
            .add_systems(OnExit(AppState::VisualizationIco), despawn_scene);
    }
}

#[derive(Component)]
struct IcoScene;

#[derive(Asset, AsBindGroup, TypePath, Debug, Clone)]
#[repr(C)]
pub struct IcoMaterial {
    #[uniform(0)]
    pub color: Vec4, // r, g, b, a
    #[uniform(0)]
    pub resolution_mouse: Vec4, // x=width, y=height, z=mouseX, w=mouseY
    #[uniform(0)]
    pub time_params: Vec4, // x=time, y=speed, z=ZOOM (camera scale), w=unused
    #[uniform(0)]
    pub audio_params: Vec4, // x=bass, y=mid, z=treble, w=flux
}

impl Material2d for IcoMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/ico_shader.wgsl".into()
    }
}

fn setup_ico_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<IcoMaterial>>,
    config: Res<VisualsConfig>,
) {
    let quad_handle = meshes.add(Rectangle::new(1.0, 1.0));

    // Initialize with default values
    let material_handle = materials.add(IcoMaterial {
        color: Vec4::from(config.ico_color.as_linear_rgba_f32()),
        resolution_mouse: Vec4::new(800.0, 600.0, 0.0, 0.0),
        time_params: Vec4::new(0.0, config.ico_speed, 1.0, 0.0),
        audio_params: Vec4::ZERO,
    });

    commands.spawn((
        MaterialMesh2dBundle {
            mesh: quad_handle.into(),
            material: material_handle,
            // Very large quad to cover the screen
            transform: Transform::from_xyz(0.0, 0.0, 0.0).with_scale(Vec3::splat(10_000.0)),
            ..default()
        },
        IcoScene,
    ));
}

fn update_ico_material(
    time: Res<Time>,
    config: Res<VisualsConfig>,
    audio_analysis: Res<AudioAnalysis>,
    mut materials: ResMut<Assets<IcoMaterial>>,
    q_window: Query<&Window, With<PrimaryWindow>>,
    q_camera: Query<&OrthographicProjection, With<MainCamera2D>>,
    q_ico: Query<&Handle<IcoMaterial>, With<IcoScene>>,
) {
    let Ok(window) = q_window.get_single() else {
        return;
    };

    let width = window.resolution.physical_width() as f32;
    let height = window.resolution.physical_height() as f32;
    let mouse = window.cursor_position().unwrap_or(Vec2::ZERO);

    let zoom_level = if let Ok(projection) = q_camera.get_single() {
        projection.scale
    } else {
        1.0
    };

    let sensitivity = config.bass_sensitivity * 0.03;

    for handle in &q_ico {
        if let Some(material) = materials.get_mut(handle) {
            material.color = Vec4::from(config.ico_color.as_linear_rgba_f32());

            material.resolution_mouse = Vec4::new(width, height, mouse.x, height - mouse.y);

            material.time_params.x = time.elapsed_seconds();
            material.time_params.y = config.ico_speed;
            material.time_params.z = zoom_level;

            material.audio_params = Vec4::new(
                audio_analysis.bass * sensitivity,
                audio_analysis.mid * sensitivity,
                audio_analysis.treble * sensitivity,
                audio_analysis.flux * sensitivity,
            );
        }
    }
}

fn despawn_scene(mut commands: Commands, scene_query: Query<Entity, With<IcoScene>>) {
    if let Ok(entity) = scene_query.get_single() {
        commands.entity(entity).despawn_recursive();
    }
}
