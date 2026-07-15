// src/main.rs

// --- Module declarations ---
mod audio;
#[cfg(target_arch = "wasm32")]
mod audio_web;
mod camera;
mod config;
mod ui;
mod viz_2d;
mod viz_3d;
mod viz_circular;
mod viz_disc;
mod viz_ico;
mod viz_matrix;
mod viz_orb;
mod viz_particles;
mod viz_starfield;
mod viz_waveform;

// --- Plugin Imports ---
use crate::audio::{
    AudioAnalysis, AudioPlugin, AudioSource, PlaybackInfo, PlaybackPosition, SelectedAudioSource,
};
use crate::camera::CameraPlugin;
use crate::config::VisualsConfig;
use crate::ui::{UiPlugin, UiVisibility};
use crate::viz_2d::Viz2DPlugin;
use crate::viz_3d::Viz3DPlugin;
use crate::viz_circular::VizCircularPlugin;
use crate::viz_disc::VizDiscPlugin;
use crate::viz_ico::VizIcoPlugin;
use crate::viz_matrix::VizMatrixPlugin;
use crate::viz_orb::VizOrbPlugin;
use crate::viz_particles::VizParticlesPlugin;
use crate::viz_starfield::VizStarfieldPlugin;
use crate::viz_waveform::VizWaveformPlugin;
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use bevy::prelude::*;
use bevy_egui::EguiPlugin;

#[cfg(not(target_arch = "wasm32"))]
use crate::audio::MicStream;
#[cfg(not(target_arch = "wasm32"))]
use rodio::{OutputStream, Sink};

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum AppState {
    #[default]
    MainMenu,
    MicSelection,
    Visualization3D,
    Visualization2D,
    VisualizationOrb,
    VisualizationDisc,
    VisualizationIco,
    VisualizationWaveform,
    VisualizationParticles,
    VisualizationCircular,
    VisualizationStarfield,
    VisualizationMatrix,
}

impl AppState {
    pub fn is_visualization(&self) -> bool {
        matches!(
            self,
            AppState::Visualization2D
                | AppState::Visualization3D
                | AppState::VisualizationOrb
                | AppState::VisualizationDisc
                | AppState::VisualizationIco
                | AppState::VisualizationWaveform
                | AppState::VisualizationParticles
                | AppState::VisualizationCircular
                | AppState::VisualizationStarfield
                | AppState::VisualizationMatrix
        )
    }
}

#[derive(Resource, Debug, Clone)]
pub struct ActiveVisualization(pub AppState);

impl Default for ActiveVisualization {
    fn default() -> Self {
        Self(AppState::Visualization3D)
    }
}

#[derive(Resource, Debug)]
pub struct VisualizationEnabled(pub bool);

impl Default for VisualizationEnabled {
    fn default() -> Self {
        Self(true)
    }
}

pub fn in_any_visualization_state(state: Option<Res<State<AppState>>>) -> bool {
    state.map(|s| s.get().is_visualization()).unwrap_or(false)
}

fn main() {
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();

    let mut app = App::new();

    // --- Platform-specific default plugins ---
    #[cfg(not(target_arch = "wasm32"))]
    app.add_plugins(DefaultPlugins);

    #[cfg(target_arch = "wasm32")]
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            canvas: Some("#bevy-canvas".to_string()),
            ..default()
        }),
        ..default()
    }));

    // --- Platform-specific audio I/O resources ---
    #[cfg(not(target_arch = "wasm32"))]
    {
        let (stream, stream_handle) = match OutputStream::try_default() {
            Ok(result) => result,
            Err(e) => {
                eprintln!("Fatal: No audio output device found: {e}");
                std::process::exit(1);
            }
        };

        let sink = match Sink::try_new(&stream_handle) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Fatal: Failed to create audio sink: {e}");
                std::process::exit(1);
            }
        };

        app.insert_non_send_resource(stream)
            .insert_non_send_resource(sink)
            .insert_non_send_resource(MicStream(None));
    }

    // --- Shared resources and plugins ---
    app.init_resource::<VisualsConfig>()
        .init_resource::<SelectedAudioSource>()
        .init_resource::<VisualizationEnabled>()
        .init_resource::<ActiveVisualization>()
        .init_resource::<PlaybackInfo>()
        .init_resource::<PlaybackPosition>()
        .init_resource::<UiVisibility>()
        .init_state::<AppState>()
        .insert_resource(ClearColor(VisualsConfig::default().bg_color))
        .add_plugins((
            FrameTimeDiagnosticsPlugin,
            EguiPlugin,
            AudioPlugin,
            UiPlugin,
            Viz2DPlugin,
            Viz3DPlugin,
            VizOrbPlugin,
            CameraPlugin,
            VizDiscPlugin,
            VizIcoPlugin,
            VizWaveformPlugin,
            VizParticlesPlugin,
            VizCircularPlugin,
            VizStarfieldPlugin,
            VizMatrixPlugin,
        ))
        .add_systems(
            Update,
            (update_background_color, handle_file_drop).run_if(in_any_visualization_state),
        )
        .run();
}

fn update_background_color(
    config: Res<VisualsConfig>,
    audio: Res<AudioAnalysis>,
    mut clear_color: ResMut<ClearColor>,
) {
    if config.bg_pulse_enabled {
        let pulse = audio.bass * config.bg_pulse_intensity;
        let r = (config.bg_color.r() + pulse).min(1.0);
        let g = (config.bg_color.g() + pulse).min(1.0);
        let b = (config.bg_color.b() + pulse).min(1.0);
        clear_color.0 = Color::rgb(r, g, b);
    } else if config.is_changed() {
        clear_color.0 = config.bg_color;
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn handle_file_drop(
    mut events: EventReader<FileDragAndDrop>,
    mut selected_source: ResMut<SelectedAudioSource>,
) {
    for event in events.read() {
        if let FileDragAndDrop::DroppedFile { path_buf, .. } = event {
            if let Some(ext) = path_buf.extension() {
                let ext = ext.to_string_lossy().to_lowercase();
                if ext == "mp3" || ext == "wav" || ext == "ogg" || ext == "flac" {
                    selected_source.0 = AudioSource::File(path_buf.clone());
                }
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn handle_file_drop() {}
