// src/viz_waveform.rs

use crate::{
    audio::{AudioSamples, MicAudioBuffer, AudioSource, SelectedAudioSource},
    config::VisualsConfig,
    AppState, VisualizationEnabled,
};
use bevy::prelude::*;

pub struct VizWaveformPlugin;

#[derive(Component)]
struct WaveformScene;

impl Plugin for VizWaveformPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::VisualizationWaveform), setup_waveform)
            .add_systems(
                Update,
                draw_waveform
                    .run_if(in_state(AppState::VisualizationWaveform))
                    .run_if(|viz_enabled: Res<VisualizationEnabled>| viz_enabled.0),
            )
            .add_systems(OnExit(AppState::VisualizationWaveform), despawn_waveform);
    }
}

fn setup_waveform(mut commands: Commands) {
    commands.spawn((SpatialBundle::default(), WaveformScene));
}

fn despawn_waveform(mut commands: Commands, query: Query<Entity, With<WaveformScene>>) {
    if let Ok(entity) = query.get_single() {
        commands.entity(entity).despawn_recursive();
    }
}

fn draw_waveform(
    mut gizmos: Gizmos,
    audio_samples: Res<AudioSamples>,
    mic_buffer: Res<MicAudioBuffer>,
    audio_source: Res<SelectedAudioSource>,
    config: Res<VisualsConfig>,
) {
    let samples: &[f32] = match &audio_source.0 {
        AudioSource::File(_) => audio_samples.0.as_slices().0,
        AudioSource::Microphone => mic_buffer.0.as_slices().0,
        AudioSource::None => return,
    };

    if samples.len() < 2 {
        return;
    }

    let display_samples = samples.len().min(2048);
    let width = config.waveform_width;
    let height = config.waveform_height;
    let step = display_samples as f32 / width;

    let color = Color::rgb(
        config.waveform_color.r(),
        config.waveform_color.g(),
        config.waveform_color.b(),
    );

    let mut prev = Vec2::new(
        -width / 2.0,
        samples[0] * height * config.bass_sensitivity,
    );

    for i in 1..(width as usize) {
        let sample_idx = (i as f32 * step) as usize;
        if sample_idx >= display_samples {
            break;
        }
        let x = -width / 2.0 + i as f32;
        let y = samples[sample_idx] * height * config.bass_sensitivity;
        let current = Vec2::new(x, y);
        gizmos.line_2d(prev, current, color);
        prev = current;
    }
}
