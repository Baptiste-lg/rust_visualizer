// src/main.rs

// --- Module declarations ---
mod audio;
mod camera;
mod config;
mod ui;
mod viz_2d;
mod viz_3d;
mod viz_disc;
mod viz_ico;
mod viz_orb;

// --- Plugin Imports ---
use crate::audio::{AudioPlugin, MicStream, PlaybackInfo, PlaybackPosition, SelectedAudioSource};
use crate::camera::CameraPlugin;
use crate::config::VisualsConfig;
use crate::ui::{UiPlugin, UiVisibility};
use crate::viz_2d::Viz2DPlugin;
use crate::viz_3d::Viz3DPlugin;
use crate::viz_disc::VizDiscPlugin;
use crate::viz_ico::VizIcoPlugin;
use crate::viz_orb::VizOrbPlugin;

use bevy::prelude::*;
use bevy_egui::EguiPlugin;
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
    let mut app = App::new();

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

    app.add_plugins(DefaultPlugins)
        .insert_non_send_resource(stream)
        .insert_non_send_resource(sink)
        .insert_non_send_resource(MicStream(None))
        .init_resource::<VisualsConfig>()
        .init_resource::<SelectedAudioSource>()
        .init_resource::<VisualizationEnabled>()
        .init_resource::<ActiveVisualization>()
        .init_resource::<PlaybackInfo>()
        .init_resource::<PlaybackPosition>()
        .init_resource::<UiVisibility>()
        .init_state::<AppState>()
        .add_plugins((
            EguiPlugin,
            AudioPlugin,
            UiPlugin,
            Viz2DPlugin,
            Viz3DPlugin,
            VizOrbPlugin,
            CameraPlugin,
            VizDiscPlugin,
            VizIcoPlugin,
        ))
        .run();
}
