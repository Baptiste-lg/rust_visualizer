// src/config.rs

use bevy::prelude::*;

// A resource that holds all the configurable parameters for the visualizations.
// This allows users to tweak the visuals in real-time through the UI.
#[derive(Resource, Clone)]
pub struct VisualsConfig {
    // --- General Settings ---
    pub bass_sensitivity: f32,
    pub num_bands: usize,
    pub details_panel_enabled: bool,

    // --- Bloom Settings ---
    pub bloom_enabled: bool,
    pub bloom_intensity: f32,
    pub bloom_threshold: f32,
    pub bloom_color: Color,

    // --- 2D Visualizer ---
    pub viz2d_inactive_color: Color,
    pub viz2d_active_color: Color,

    // --- 3D Visualizer ---
    pub spread_enabled: bool,
    pub viz3d_base_color: Color,
    pub viz3d_column_size: usize,

    // --- Orb Visualizer ---
    pub orb_base_color: Color,
    pub orb_peak_color: Color,
    pub orb_noise_speed: f32,
    pub orb_noise_frequency: f32,
    pub orb_treble_influence: f32,

    // --- Disc Visualizer Settings ---
    pub disc_color: Color,
    pub disc_radius: f32,
    pub disc_line_thickness: f32,
    pub disc_iterations: i32,
    pub disc_speed: f32,
    pub disc_center_radius_factor: f32,

    // --- Ico Visualizer Settings ---
    pub ico_speed: f32,
    pub ico_color: Color,

    // --- Background Settings ---
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

            // --- Background Defaults ---
            bg_color: Color::rgb(0.05, 0.05, 0.1),
            bg_pulse_enabled: false,
            bg_pulse_intensity: 0.3,
        }
    }
}
