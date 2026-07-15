// src/config.rs

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

mod color_serde {
    use bevy::prelude::Color;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    #[derive(Serialize, Deserialize)]
    struct Rgba {
        r: f32,
        g: f32,
        b: f32,
        a: f32,
    }

    pub fn serialize<S: Serializer>(color: &Color, s: S) -> Result<S::Ok, S::Error> {
        Rgba {
            r: color.r(),
            g: color.g(),
            b: color.b(),
            a: color.a(),
        }
        .serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Color, D::Error> {
        let rgba = Rgba::deserialize(d)?;
        Ok(Color::rgba(rgba.r, rgba.g, rgba.b, rgba.a))
    }
}

// A resource that holds all the configurable parameters for the visualizations.
// This allows users to tweak the visuals in real-time through the UI.
#[derive(Resource, Clone, Serialize, Deserialize)]
pub struct VisualsConfig {
    // --- General Settings ---
    pub bass_sensitivity: f32,
    pub num_bands: usize,
    pub details_panel_enabled: bool,

    // --- Bloom Settings ---
    pub bloom_enabled: bool,
    pub bloom_intensity: f32,
    pub bloom_threshold: f32,
    #[serde(with = "color_serde")]
    pub bloom_color: Color,

    // --- 2D Visualizer ---
    #[serde(with = "color_serde")]
    pub viz2d_inactive_color: Color,
    #[serde(with = "color_serde")]
    pub viz2d_active_color: Color,

    // --- 3D Visualizer ---
    pub spread_enabled: bool,
    #[serde(with = "color_serde")]
    pub viz3d_base_color: Color,
    pub viz3d_column_size: usize,

    // --- Orb Visualizer ---
    #[serde(with = "color_serde")]
    pub orb_base_color: Color,
    #[serde(with = "color_serde")]
    pub orb_peak_color: Color,
    pub orb_noise_speed: f32,
    pub orb_noise_frequency: f32,
    pub orb_treble_influence: f32,

    // --- Disc Visualizer Settings ---
    #[serde(with = "color_serde")]
    pub disc_color: Color,
    pub disc_radius: f32,
    pub disc_line_thickness: f32,
    pub disc_iterations: i32,
    pub disc_speed: f32,
    pub disc_center_radius_factor: f32,

    // --- Ico Visualizer Settings ---
    pub ico_speed: f32,
    #[serde(with = "color_serde")]
    pub ico_color: Color,

    // --- Waveform Visualizer Settings ---
    #[serde(with = "color_serde")]
    pub waveform_color: Color,
    pub waveform_width: f32,
    pub waveform_height: f32,

    // --- Background Settings ---
    #[serde(with = "color_serde")]
    pub bg_color: Color,
    pub bg_pulse_enabled: bool,
    pub bg_pulse_intensity: f32,
}

impl Default for VisualsConfig {
    fn default() -> Self {
        Self {
            // --- General ---
            bass_sensitivity: 1.0,
            num_bands: 16,
            details_panel_enabled: false,

            // --- Bloom ---
            bloom_enabled: true,
            bloom_intensity: 0.3,
            bloom_threshold: 0.8,
            bloom_color: Color::rgb(1.0, 0.2, 0.0),

            // --- 2D ---
            viz2d_inactive_color: Color::rgb(0.2, 0.2, 0.8),
            viz2d_active_color: Color::rgb(1.0, 0.3, 0.9),

            // --- 3D ---
            spread_enabled: true,
            viz3d_base_color: Color::rgb(0.8, 0.7, 0.6),
            viz3d_column_size: 8,

            // --- Orb ---
            orb_base_color: Color::rgb(0.1, 0.1, 0.7),
            orb_peak_color: Color::rgb(1.0, 0.0, 1.0),
            orb_noise_speed: 1.0,
            orb_noise_frequency: 2.0,
            orb_treble_influence: 0.3,

            // --- Disc Visualizer Defaults ---
            disc_color: Color::rgb(1.0, 0.8, 0.2),
            disc_radius: 0.8,
            disc_line_thickness: 0.07,
            disc_iterations: 35,
            disc_speed: 1.0,
            disc_center_radius_factor: 1.0,

            // --- Ico Visualizer Defaults ---
            ico_speed: 0.5,
            ico_color: Color::rgb(0.5, 0.8, 0.9),

            // --- Waveform Defaults ---
            waveform_color: Color::rgb(0.2, 0.8, 1.0),
            waveform_width: 800.0,
            waveform_height: 200.0,

            // --- Background Defaults ---
            bg_color: Color::rgb(0.05, 0.05, 0.1),
            bg_pulse_enabled: false,
            bg_pulse_intensity: 0.3,
        }
    }
}

impl VisualsConfig {
    pub fn preset_chill() -> Self {
        Self {
            bass_sensitivity: 0.6,
            bloom_enabled: true,
            bloom_intensity: 0.15,
            bloom_threshold: 1.2,
            bloom_color: Color::rgb(0.3, 0.1, 0.5),
            viz2d_inactive_color: Color::rgb(0.1, 0.1, 0.3),
            viz2d_active_color: Color::rgb(0.4, 0.3, 0.8),
            viz3d_base_color: Color::rgb(0.3, 0.3, 0.5),
            orb_base_color: Color::rgb(0.1, 0.05, 0.3),
            orb_peak_color: Color::rgb(0.5, 0.2, 0.8),
            orb_noise_speed: 0.4,
            orb_noise_frequency: 1.5,
            disc_color: Color::rgb(0.4, 0.3, 0.7),
            disc_speed: 0.3,
            ico_color: Color::rgb(0.4, 0.4, 0.7),
            ico_speed: 0.2,
            bg_color: Color::rgb(0.02, 0.02, 0.06),
            bg_pulse_enabled: true,
            bg_pulse_intensity: 0.1,
            ..Default::default()
        }
    }

    pub fn preset_energetic() -> Self {
        Self {
            bass_sensitivity: 3.0,
            bloom_enabled: true,
            bloom_intensity: 0.6,
            bloom_threshold: 0.4,
            bloom_color: Color::rgb(1.0, 0.4, 0.0),
            viz2d_inactive_color: Color::rgb(0.8, 0.1, 0.1),
            viz2d_active_color: Color::rgb(1.0, 1.0, 0.0),
            viz3d_base_color: Color::rgb(1.0, 0.3, 0.1),
            orb_base_color: Color::rgb(0.8, 0.1, 0.0),
            orb_peak_color: Color::rgb(1.0, 1.0, 0.0),
            orb_noise_speed: 3.0,
            orb_noise_frequency: 4.0,
            disc_color: Color::rgb(1.0, 0.3, 0.1),
            disc_speed: 2.5,
            ico_color: Color::rgb(1.0, 0.5, 0.2),
            ico_speed: 1.5,
            bg_color: Color::rgb(0.08, 0.02, 0.02),
            bg_pulse_enabled: true,
            bg_pulse_intensity: 0.5,
            ..Default::default()
        }
    }

    pub fn preset_neon() -> Self {
        Self {
            bass_sensitivity: 2.0,
            bloom_enabled: true,
            bloom_intensity: 0.5,
            bloom_threshold: 0.5,
            bloom_color: Color::rgb(0.0, 1.0, 0.5),
            viz2d_inactive_color: Color::rgb(0.0, 0.2, 0.4),
            viz2d_active_color: Color::rgb(0.0, 1.0, 1.0),
            viz3d_base_color: Color::rgb(0.0, 0.8, 0.6),
            orb_base_color: Color::rgb(0.0, 0.2, 0.5),
            orb_peak_color: Color::rgb(0.0, 1.0, 0.8),
            orb_noise_speed: 1.5,
            orb_noise_frequency: 3.0,
            disc_color: Color::rgb(0.0, 1.0, 0.6),
            disc_speed: 1.5,
            ico_color: Color::rgb(0.0, 1.0, 0.8),
            ico_speed: 0.8,
            bg_color: Color::rgb(0.0, 0.02, 0.05),
            bg_pulse_enabled: true,
            bg_pulse_intensity: 0.2,
            ..Default::default()
        }
    }

    pub fn preset_monochrome() -> Self {
        Self {
            bass_sensitivity: 1.5,
            bloom_enabled: false,
            bloom_color: Color::rgb(1.0, 1.0, 1.0),
            viz2d_inactive_color: Color::rgb(0.2, 0.2, 0.2),
            viz2d_active_color: Color::rgb(1.0, 1.0, 1.0),
            viz3d_base_color: Color::rgb(0.6, 0.6, 0.6),
            orb_base_color: Color::rgb(0.2, 0.2, 0.2),
            orb_peak_color: Color::rgb(1.0, 1.0, 1.0),
            disc_color: Color::rgb(0.9, 0.9, 0.9),
            ico_color: Color::rgb(0.8, 0.8, 0.8),
            bg_color: Color::rgb(0.0, 0.0, 0.0),
            bg_pulse_enabled: false,
            ..Default::default()
        }
    }
}
