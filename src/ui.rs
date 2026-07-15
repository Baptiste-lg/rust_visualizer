// src/ui.rs

use crate::audio::{
    AudioAnalysis, AudioSource, PlaybackInfo, PlaybackPosition, PlaybackStatus, SelectedAudioSource,
};
use crate::config::VisualsConfig;
use crate::{in_any_visualization_state, ActiveVisualization, AppState, VisualizationEnabled};
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

#[derive(SystemParam)]
struct PlaybackParams<'w> {
    selected_source: ResMut<'w, SelectedAudioSource>,
    playback_info: ResMut<'w, PlaybackInfo>,
    playback_pos: ResMut<'w, PlaybackPosition>,
}

#[derive(SystemParam)]
struct VizStateParams<'w> {
    viz_enabled: ResMut<'w, VisualizationEnabled>,
    app_state: Res<'w, State<AppState>>,
    next_app_state: ResMut<'w, NextState<AppState>>,
    active_viz: ResMut<'w, ActiveVisualization>,
    fade: ResMut<'w, TransitionFade>,
}
use bevy_egui::egui::color_picker;
use bevy_egui::{egui, EguiContexts, EguiSet};
use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
use crate::audio::SelectedMic;
#[cfg(not(target_arch = "wasm32"))]
use cpal::traits::{DeviceTrait, HostTrait};

// A resource to know if the UI is shown or hidden
#[derive(Resource)]
pub struct UiVisibility {
    pub visible: bool,
    pub hint_timer: Timer,
}

#[derive(Resource, Default)]
pub struct TransitionFade {
    pub alpha: f32,
}

impl Default for UiVisibility {
    fn default() -> Self {
        Self {
            visible: true,
            hint_timer: Timer::from_seconds(5.0, TimerMode::Once),
        }
    }
}

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::MainMenu), setup_main_menu)
            .add_systems(
                Update,
                menu_button_interaction.run_if(in_state(AppState::MainMenu)),
            )
            .add_systems(OnExit(AppState::MainMenu), cleanup_menu);

        // Mic selection menu — native only (browser handles device selection)
        #[cfg(not(target_arch = "wasm32"))]
        {
            app.add_systems(OnEnter(AppState::MicSelection), setup_mic_selection_menu)
                .add_systems(
                    Update,
                    mic_selection_interaction.run_if(in_state(AppState::MicSelection)),
                );
        }
        app.add_systems(OnExit(AppState::MicSelection), cleanup_menu);

        app.init_resource::<TransitionFade>().add_systems(
            Update,
            (
                toggle_ui_visibility,
                main_ui_layout,
                fps_overlay,
                render_transition_fade,
            )
                .after(EguiSet::InitContexts)
                .run_if(in_any_visualization_state),
        );
    }
}

// --- Main Menu Components ---
#[derive(Component)]
enum MenuButtonAction {
    Start,
    #[cfg(not(target_arch = "wasm32"))]
    ToMicSelection,
}

#[derive(Component)]
struct MainMenuUI;

#[cfg(not(target_arch = "wasm32"))]
#[derive(Component)]
struct MicDeviceButton(String);

// --- UI Toggle & Keyboard Shortcuts ---
fn toggle_ui_visibility(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut ui_viz: ResMut<UiVisibility>,
    mut next_app_state: ResMut<NextState<AppState>>,
    mut active_viz: ResMut<ActiveVisualization>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    mut fade: ResMut<TransitionFade>,
) {
    if keyboard.just_pressed(KeyCode::KeyH) {
        ui_viz.visible = !ui_viz.visible;
        if !ui_viz.visible {
            ui_viz.hint_timer.reset();
        }
    }

    // Screenshot
    #[cfg(target_arch = "wasm32")]
    if keyboard.just_pressed(KeyCode::KeyP) {
        crate::audio_web::trigger_screenshot();
    }

    // Fullscreen toggle
    if keyboard.just_pressed(KeyCode::KeyF) {
        if let Ok(mut window) = windows.get_single_mut() {
            window.mode = match window.mode {
                bevy::window::WindowMode::Windowed => {
                    bevy::window::WindowMode::BorderlessFullscreen
                }
                _ => bevy::window::WindowMode::Windowed,
            };
        }
    }

    // Number keys to switch visualization
    let mappings = [
        (KeyCode::Digit1, AppState::Visualization2D),
        (KeyCode::Digit2, AppState::Visualization3D),
        (KeyCode::Digit3, AppState::VisualizationOrb),
        (KeyCode::Digit4, AppState::VisualizationDisc),
        (KeyCode::Digit5, AppState::VisualizationIco),
        (KeyCode::Digit6, AppState::VisualizationWaveform),
        (KeyCode::Digit7, AppState::VisualizationParticles),
        (KeyCode::Digit8, AppState::VisualizationCircular),
        (KeyCode::Digit9, AppState::VisualizationStarfield),
        (KeyCode::Digit0, AppState::VisualizationMatrix),
    ];
    for (key, state) in mappings {
        if keyboard.just_pressed(key) {
            fade.alpha = 1.0;
            next_app_state.set(state.clone());
            active_viz.0 = state;
            break;
        }
    }
}

// --- Main UI System (Layout & Content) ---
#[allow(clippy::too_many_arguments)]
fn main_ui_layout(
    mut contexts: EguiContexts,
    mut config: ResMut<VisualsConfig>,
    mut playback: PlaybackParams,
    mut viz_state: VizStateParams,
    mut ui_visibility: ResMut<UiVisibility>,
    time: Res<Time>,
    audio_analysis: Res<AudioAnalysis>,
    q_windows: Query<Entity, With<PrimaryWindow>>,
    mut beat_flash: Local<f32>,
) {
    if q_windows.get_single().is_err() {
        return;
    }

    let ctx = contexts.ctx_mut();

    // Beat flash overlay
    if audio_analysis.beat_detected {
        *beat_flash = 0.4;
    }
    if *beat_flash > 0.0 {
        let alpha = (*beat_flash * 255.0) as u8;
        egui::Area::new("beat_flash_overlay".into())
            .anchor(egui::Align2::LEFT_TOP, egui::vec2(0.0, 0.0))
            .interactable(false)
            .show(ctx, |ui| {
                let screen = ui.ctx().screen_rect();
                ui.painter()
                    .rect_filled(screen, 0.0_f32, egui::Color32::from_white_alpha(alpha));
            });
        *beat_flash = (*beat_flash - time.delta_seconds() * 3.0).max(0.0);
    }

    // 1. LOGIC WHEN UI IS HIDDEN
    if !ui_visibility.visible {
        ui_visibility.hint_timer.tick(time.delta());
        if !ui_visibility.hint_timer.finished() {
            egui::Area::new("ui_hidden_hint".into())
                .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 10.0))
                .show(ctx, |ui| {
                    ui.visuals_mut().widgets.noninteractive.bg_fill =
                        egui::Color32::from_black_alpha(150);
                    ui.visuals_mut().widgets.noninteractive.fg_stroke =
                        egui::Stroke::new(1.0_f32, egui::Color32::WHITE);
                    egui::Frame::default().inner_margin(8.0).show(ui, |ui| {
                        ui.label(egui::RichText::new("Press 'H' to Show UI").size(16.0));
                    });
                });
        }
        return;
    }

    // 2. LOGIC WHEN UI IS VISIBLE (Panels)
    let current_state = viz_state.app_state.get();

    // --- LEFT PANEL: Active Visualizer Settings ---
    egui::SidePanel::left("viz_settings_panel")
        .resizable(true)
        .default_width(250.0)
        .show(ctx, |ui| {
            ui.add_space(10.0);
            ui.heading("🎨 Visualizer Settings");
            ui.separator();

            ui.label("Amplitude Sensitivity");
            ui.add(egui::Slider::new(&mut config.bass_sensitivity, 0.1..=10.0));

            ui.separator();
            ui.heading("🌌 Background");
            ui.label("Color");
            color_picker_widget(ui, &mut config.bg_color);
            ui.checkbox(&mut config.bg_pulse_enabled, "Pulse with Bass");
            if config.bg_pulse_enabled {
                ui.label("Pulse Intensity");
                ui.add(egui::Slider::new(&mut config.bg_pulse_intensity, 0.0..=1.0));
            }

            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| match current_state {
                AppState::Visualization2D => {
                    ui.label("Inactive Color");
                    color_picker_widget(ui, &mut config.viz2d_inactive_color);
                    ui.label("Active Color");
                    color_picker_widget(ui, &mut config.viz2d_active_color);
                    ui.separator();
                    ui.label("Frequency Bands (Rebuilds Grid)");
                    ui.add(egui::Slider::new(&mut config.num_bands, 4..=64));
                }
                AppState::Visualization3D => {
                    ui.checkbox(&mut config.spread_enabled, "Spread Effect");
                    ui.label("Column Size");
                    ui.add(egui::Slider::new(&mut config.viz3d_column_size, 1..=16));
                    ui.label("Cube Base Color");
                    color_picker_widget(ui, &mut config.viz3d_base_color);
                    ui.separator();
                    ui.label("Frequency Bands (Rebuilds Grid)");
                    ui.add(egui::Slider::new(&mut config.num_bands, 4..=32));
                    ui.separator();
                    render_bloom_ui(ui, &mut config);
                }
                AppState::VisualizationOrb => {
                    ui.label("Base Color");
                    color_picker_widget(ui, &mut config.orb_base_color);
                    ui.label("Peak Color");
                    color_picker_widget(ui, &mut config.orb_peak_color);
                    ui.separator();
                    ui.label("Noise Speed");
                    ui.add(egui::Slider::new(&mut config.orb_noise_speed, 0.1..=5.0));
                    ui.label("Noise Frequency");
                    ui.add(egui::Slider::new(
                        &mut config.orb_noise_frequency,
                        0.5..=10.0,
                    ));
                    ui.label("Treble Influence");
                    ui.add(egui::Slider::new(
                        &mut config.orb_treble_influence,
                        0.0..=1.0,
                    ));
                    ui.separator();
                    render_bloom_ui(ui, &mut config);
                }
                AppState::VisualizationDisc => {
                    ui.label("Disc Color");
                    color_picker_widget(ui, &mut config.disc_color);
                    ui.label("Radius");
                    ui.add(egui::Slider::new(&mut config.disc_radius, 0.1..=2.0));
                    ui.label("Line Thickness");
                    ui.add(egui::Slider::new(
                        &mut config.disc_line_thickness,
                        0.01..=0.5,
                    ));
                    ui.label("Iterations (Echoes)");
                    ui.add(egui::Slider::new(&mut config.disc_iterations, 1..=50));
                    ui.label("Rotation Speed");
                    ui.add(egui::Slider::new(&mut config.disc_speed, -5.0..=5.0));
                    ui.label("Center Factor");
                    ui.add(egui::Slider::new(
                        &mut config.disc_center_radius_factor,
                        -1.0..=2.0,
                    ));
                }
                AppState::VisualizationIco => {
                    ui.label("Metallic Color");
                    color_picker_widget(ui, &mut config.ico_color);
                    ui.label("Rotation Speed");
                    ui.add(egui::Slider::new(&mut config.ico_speed, -3.0..=3.0));
                }
                AppState::VisualizationWaveform => {
                    ui.label("Waveform Color");
                    color_picker_widget(ui, &mut config.waveform_color);
                    ui.label("Display Width");
                    ui.add(egui::Slider::new(
                        &mut config.waveform_width,
                        200.0..=1600.0,
                    ));
                    ui.label("Vertical Scale");
                    ui.add(egui::Slider::new(&mut config.waveform_height, 50.0..=800.0));
                }
                AppState::VisualizationParticles => {
                    ui.label("Particle Color");
                    color_picker_widget(ui, &mut config.particles_color);
                    ui.label("Particles per Beat");
                    ui.add(egui::Slider::new(&mut config.particles_count, 1..=50));
                    ui.label("Particle Size");
                    ui.add(egui::Slider::new(&mut config.particles_size, 1.0..=20.0));
                    ui.label("Gravity");
                    ui.add(egui::Slider::new(
                        &mut config.particles_gravity,
                        -500.0..=0.0,
                    ));
                    ui.label("Lifetime (s)");
                    ui.add(egui::Slider::new(&mut config.particles_lifetime, 0.5..=5.0));
                }
                AppState::VisualizationCircular => {
                    ui.label("Color");
                    color_picker_widget(ui, &mut config.circular_color);
                    ui.label("Bar Count");
                    ui.add(egui::Slider::new(&mut config.circular_bar_count, 8..=128));
                    ui.label("Radius");
                    ui.add(egui::Slider::new(&mut config.circular_radius, 50.0..=400.0));
                    ui.label("Bar Width");
                    ui.add(egui::Slider::new(&mut config.circular_bar_width, 1.0..=10.0));
                    ui.label("Rotation Speed");
                    ui.add(egui::Slider::new(
                        &mut config.circular_rotation_speed,
                        -2.0..=2.0,
                    ));
                }
                AppState::VisualizationStarfield => {
                    ui.label("Star Color");
                    color_picker_widget(ui, &mut config.starfield_color);
                    ui.label("Star Count (restart to apply)");
                    ui.add(egui::Slider::new(&mut config.starfield_count, 50..=500));
                    ui.label("Speed");
                    ui.add(egui::Slider::new(&mut config.starfield_speed, 10.0..=500.0));
                    ui.label("Spread");
                    ui.add(egui::Slider::new(
                        &mut config.starfield_spread,
                        200.0..=1200.0,
                    ));
                }
                AppState::VisualizationMatrix => {
                    ui.label("Color");
                    color_picker_widget(ui, &mut config.matrix_color);
                    ui.label("Columns (restart to apply)");
                    ui.add(egui::Slider::new(&mut config.matrix_columns, 10..=80));
                    ui.label("Fall Speed");
                    ui.add(egui::Slider::new(&mut config.matrix_speed, 50.0..=500.0));
                    ui.label("Density");
                    ui.add(egui::Slider::new(&mut config.matrix_density, 0.1..=1.0));
                }
                AppState::MainMenu | AppState::MicSelection => {}
            });
        });

    // --- RIGHT PANEL: Global Controls ---
    egui::SidePanel::right("global_controls_panel")
        .resizable(true)
        .default_width(250.0)
        .show(ctx, |ui| {
            ui.add_space(10.0);
            ui.heading("🎛 Controls");
            ui.separator();

            // Presets
            ui.label("Presets:");
            ui.horizontal_wrapped(|ui| {
                if ui.button("Default").clicked() {
                    *config = VisualsConfig::default();
                }
                if ui.button("Chill").clicked() {
                    *config = VisualsConfig::preset_chill();
                }
                if ui.button("Energetic").clicked() {
                    *config = VisualsConfig::preset_energetic();
                }
                if ui.button("Neon").clicked() {
                    *config = VisualsConfig::preset_neon();
                }
                if ui.button("Mono").clicked() {
                    *config = VisualsConfig::preset_monochrome();
                }
            });
            ui.horizontal(|ui| {
                if ui.button("📤 Export").clicked() {
                    if let Ok(json) = serde_json::to_string_pretty(&*config) {
                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            if let Some(path) = rfd::FileDialog::new()
                                .set_file_name("visualizer_config.json")
                                .add_filter("JSON", &["json"])
                                .save_file()
                            {
                                if let Err(e) = std::fs::write(&path, &json) {
                                    error!("Failed to save config: {e}");
                                }
                            }
                        }
                        #[cfg(target_arch = "wasm32")]
                        {
                            ui.output_mut(|o| o.copied_text = json);
                        }
                    }
                }
                if ui.button("📥 Import").clicked() {
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("JSON", &["json"])
                            .pick_file()
                        {
                            match std::fs::read_to_string(&path) {
                                Ok(contents) => {
                                    match serde_json::from_str::<VisualsConfig>(&contents) {
                                        Ok(loaded) => *config = loaded,
                                        Err(e) => error!("Invalid config file: {e}"),
                                    }
                                }
                                Err(e) => error!("Failed to read config file: {e}"),
                            }
                        }
                    }
                }
            });
            #[cfg(target_arch = "wasm32")]
            ui.label(
                egui::RichText::new("Export copies JSON to clipboard")
                    .weak()
                    .small(),
            );

            ui.separator();

            // Visualizer Choice
            ui.label("Select Visualizer:");
            ui.horizontal_wrapped(|ui| {
                if ui
                    .selectable_label(*current_state == AppState::Visualization2D, "2D Bars")
                    .clicked()
                {
                    viz_state.next_app_state.set(AppState::Visualization2D);
                    viz_state.active_viz.0 = AppState::Visualization2D;
                }
                if ui
                    .selectable_label(*current_state == AppState::Visualization3D, "3D Cubes")
                    .clicked()
                {
                    viz_state.next_app_state.set(AppState::Visualization3D);
                    viz_state.active_viz.0 = AppState::Visualization3D;
                }
                if ui
                    .selectable_label(*current_state == AppState::VisualizationOrb, "3D Orb")
                    .clicked()
                {
                    viz_state.next_app_state.set(AppState::VisualizationOrb);
                    viz_state.active_viz.0 = AppState::VisualizationOrb;
                }
                if ui
                    .selectable_label(*current_state == AppState::VisualizationDisc, "Disc")
                    .clicked()
                {
                    viz_state.next_app_state.set(AppState::VisualizationDisc);
                    viz_state.active_viz.0 = AppState::VisualizationDisc;
                }
                if ui
                    .selectable_label(*current_state == AppState::VisualizationIco, "Ico")
                    .clicked()
                {
                    viz_state.next_app_state.set(AppState::VisualizationIco);
                    viz_state.active_viz.0 = AppState::VisualizationIco;
                }
                if ui
                    .selectable_label(
                        *current_state == AppState::VisualizationWaveform,
                        "Waveform",
                    )
                    .clicked()
                {
                    viz_state
                        .next_app_state
                        .set(AppState::VisualizationWaveform);
                    viz_state.active_viz.0 = AppState::VisualizationWaveform;
                }
                if ui
                    .selectable_label(
                        *current_state == AppState::VisualizationParticles,
                        "Particles",
                    )
                    .clicked()
                {
                    viz_state
                        .next_app_state
                        .set(AppState::VisualizationParticles);
                    viz_state.active_viz.0 = AppState::VisualizationParticles;
                }
                if ui
                    .selectable_label(
                        *current_state == AppState::VisualizationCircular,
                        "Circular",
                    )
                    .clicked()
                {
                    viz_state
                        .next_app_state
                        .set(AppState::VisualizationCircular);
                    viz_state.active_viz.0 = AppState::VisualizationCircular;
                }
                if ui
                    .selectable_label(
                        *current_state == AppState::VisualizationStarfield,
                        "Starfield",
                    )
                    .clicked()
                {
                    viz_state
                        .next_app_state
                        .set(AppState::VisualizationStarfield);
                    viz_state.active_viz.0 = AppState::VisualizationStarfield;
                }
                if ui
                    .selectable_label(
                        *current_state == AppState::VisualizationMatrix,
                        "Matrix",
                    )
                    .clicked()
                {
                    viz_state
                        .next_app_state
                        .set(AppState::VisualizationMatrix);
                    viz_state.active_viz.0 = AppState::VisualizationMatrix;
                }
            });

            ui.separator();

            // Global Toggle On/Off
            let btn_text = if viz_state.viz_enabled.0 {
                "⏹ Stop Render"
            } else {
                "▶ Start Render"
            };
            if ui.button(btn_text).clicked() {
                viz_state.viz_enabled.0 = !viz_state.viz_enabled.0;
            }

            ui.separator();
            ui.heading("🎵 Audio Source");

            // --- Microphone button ---
            if ui.button("🎤 Microphone").clicked() {
                #[cfg(target_arch = "wasm32")]
                crate::audio_web::request_microphone();

                playback.selected_source.0 = AudioSource::Microphone;
            }

            // --- File picker button ---
            if ui.button("📂 Load File").clicked() {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("audio", &["mp3", "wav"])
                        .pick_file()
                    {
                        playback.selected_source.0 = AudioSource::File(path);
                    }
                }

                #[cfg(target_arch = "wasm32")]
                crate::audio_web::request_file();
            }

            // Playback Controls (If file)
            if let AudioSource::File(_) = playback.selected_source.0 {
                ui.separator();
                ui.label("Playback:");
                ui.horizontal(|ui| {
                    let icon = if playback.playback_info.status == PlaybackStatus::Playing {
                        "⏸"
                    } else {
                        "▶"
                    };
                    if ui.button(icon).clicked() {
                        playback.playback_info.status = match playback.playback_info.status {
                            PlaybackStatus::Playing => PlaybackStatus::Paused,
                            PlaybackStatus::Paused => PlaybackStatus::Playing,
                        };
                    }

                    ui.label("Speed:");
                    ui.add(
                        egui::Slider::new(&mut playback.playback_info.speed, 0.25..=2.0).text("x"),
                    );
                });

                // Progress Bar
                if playback.playback_info.duration > Duration::ZERO {
                    let total = playback.playback_info.duration.as_secs_f32();
                    let mut pos = playback.playback_pos.position.as_secs_f32();
                    let label = format!("{:.0}s / {:.0}s", pos, total);
                    if ui
                        .add(
                            egui::Slider::new(&mut pos, 0.0..=total)
                                .show_value(false)
                                .text(label),
                        )
                        .changed()
                    {
                        playback.playback_info.seek_to = Some(pos);
                    }
                }
            }

            ui.separator();
            ui.checkbox(&mut config.details_panel_enabled, "Show Analysis Data");

            if config.details_panel_enabled {
                ui.separator();
                ui.label(egui::RichText::new("Analysis Data").strong());
                ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);
                ui.label(format!("Volume: {:.3}", audio_analysis.volume));
                ui.label(format!("Bass:   {:.2}", audio_analysis.bass));
                ui.label(format!("Mid:    {:.2}", audio_analysis.mid));
                ui.label(format!("Treble: {:.2}", audio_analysis.treble));
                ui.label(format!("Flux:   {:.2}", audio_analysis.flux));
                ui.separator();
                let beat_indicator = if audio_analysis.beat_detected {
                    "BEAT!"
                } else {
                    "----"
                };
                ui.label(egui::RichText::new(beat_indicator).strong().color(
                    if audio_analysis.beat_detected {
                        egui::Color32::YELLOW
                    } else {
                        egui::Color32::GRAY
                    },
                ));
                if audio_analysis.bpm > 0.0 {
                    ui.label(format!("BPM:    {:.0}", audio_analysis.bpm));
                }
            }

            ui.add_space(20.0);
            ui.separator();
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new(
                        "H: Hide UI | F: Fullscreen | P: Screenshot | 1-0: Switch Viz",
                    )
                    .weak()
                    .italics(),
                );
            });
        });
}

fn render_bloom_ui(ui: &mut egui::Ui, config: &mut VisualsConfig) {
    ui.heading("✨ Bloom");
    ui.checkbox(&mut config.bloom_enabled, "Enable");
    if config.bloom_enabled {
        ui.label("Intensity");
        ui.add(egui::Slider::new(&mut config.bloom_intensity, 0.0..=1.0));
        ui.label("Threshold");
        ui.add(egui::Slider::new(&mut config.bloom_threshold, 0.0..=2.0));
        ui.label("Tint");
        color_picker_widget(ui, &mut config.bloom_color);
    }
}

fn color_picker_widget(ui: &mut egui::Ui, color: &mut Color) {
    let rgba_array = [color.r(), color.g(), color.b(), color.a()];

    let mut egui_color = egui::Rgba::from_rgba_unmultiplied(
        rgba_array[0],
        rgba_array[1],
        rgba_array[2],
        rgba_array[3],
    );

    if color_picker::color_edit_button_rgba(ui, &mut egui_color, color_picker::Alpha::Opaque)
        .changed()
    {
        let result = egui_color.to_rgba_unmultiplied();
        *color = Color::rgba(result[0], result[1], result[2], result[3]);
    }
}

// --- Setup Main Menu ---
fn setup_main_menu(mut commands: Commands) {
    commands.spawn((Camera2dBundle::default(), MainMenuUI));
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(15.0),
                    ..default()
                },
                ..default()
            },
            MainMenuUI,
        ))
        .with_children(|parent| {
            create_menu_button(parent, "Start Visualization", MenuButtonAction::Start);

            #[cfg(not(target_arch = "wasm32"))]
            create_menu_button(
                parent,
                "Select Microphone",
                MenuButtonAction::ToMicSelection,
            );
        });
}

#[allow(clippy::type_complexity)]
fn menu_button_interaction(
    mut button_query: Query<
        (&Interaction, &MenuButtonAction),
        (Changed<Interaction>, With<Button>),
    >,
    mut next_app_state: ResMut<NextState<AppState>>,
    active_viz: Res<ActiveVisualization>,
) {
    for (interaction, action) in &mut button_query {
        if *interaction == Interaction::Pressed {
            match action {
                MenuButtonAction::Start => {
                    next_app_state.set(active_viz.0.clone());
                }
                #[cfg(not(target_arch = "wasm32"))]
                MenuButtonAction::ToMicSelection => {
                    next_app_state.set(AppState::MicSelection);
                }
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn setup_mic_selection_menu(mut commands: Commands) {
    commands.spawn((Camera2dBundle::default(), MainMenuUI));
    let mut root = commands.spawn((
        NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(10.0),
                ..default()
            },
            ..default()
        },
        MainMenuUI,
    ));

    root.with_children(|parent| {
        parent.spawn(TextBundle::from_section(
            "Select an Input Device",
            TextStyle {
                font_size: 32.0,
                color: Color::WHITE,
                ..default()
            },
        ));
    });

    let host = cpal::default_host();
    if let Ok(devices) = host.input_devices() {
        root.with_children(|parent| {
            for device in devices {
                if let Ok(name) = device.name() {
                    parent
                        .spawn((
                            ButtonBundle {
                                style: Style {
                                    width: Val::Px(500.0),
                                    height: Val::Px(50.0),
                                    justify_content: JustifyContent::Center,
                                    align_items: AlignItems::Center,
                                    margin: UiRect::top(Val::Px(5.0)),
                                    ..default()
                                },
                                background_color: Color::rgb(0.2, 0.2, 0.2).into(),
                                ..default()
                            },
                            MicDeviceButton(name.clone()),
                        ))
                        .with_children(|btn| {
                            btn.spawn(TextBundle::from_section(
                                name,
                                TextStyle {
                                    font_size: 18.0,
                                    color: Color::WHITE,
                                    ..default()
                                },
                            ));
                        });
                }
            }
        });
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(clippy::type_complexity)]
fn mic_selection_interaction(
    mut button_query: Query<(&Interaction, &MicDeviceButton), (Changed<Interaction>, With<Button>)>,
    mut selected_mic: ResMut<SelectedMic>,
    mut next_app_state: ResMut<NextState<AppState>>,
) {
    for (interaction, button) in &mut button_query {
        if *interaction == Interaction::Pressed {
            selected_mic.0 = Some(button.0.clone());
            next_app_state.set(AppState::MainMenu);
        }
    }
}

fn create_menu_button(parent: &mut ChildBuilder, text: &str, action: MenuButtonAction) {
    parent
        .spawn((
            ButtonBundle {
                style: Style {
                    width: Val::Px(250.0),
                    height: Val::Px(65.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                background_color: Color::rgb(0.15, 0.15, 0.15).into(),
                ..default()
            },
            action,
        ))
        .with_children(|p| {
            p.spawn(TextBundle::from_section(
                text,
                TextStyle {
                    font_size: 24.0,
                    color: Color::WHITE,
                    ..default()
                },
            ));
        });
}

fn cleanup_menu(mut commands: Commands, ui_query: Query<Entity, With<MainMenuUI>>) {
    for entity in ui_query.iter() {
        commands.entity(entity).despawn_recursive();
    }
}

fn fps_overlay(mut contexts: EguiContexts, diagnostics: Res<DiagnosticsStore>) {
    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0);

    let ctx = contexts.ctx_mut();
    egui::Area::new("fps_overlay".into())
        .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-10.0, -10.0))
        .interactable(false)
        .show(ctx, |ui| {
            ui.label(
                egui::RichText::new(format!("{:.0} FPS", fps))
                    .color(egui::Color32::from_white_alpha(150))
                    .small(),
            );
        });
}

fn render_transition_fade(
    mut contexts: EguiContexts,
    mut fade: ResMut<TransitionFade>,
    time: Res<Time>,
    app_state: Res<State<AppState>>,
    mut last_state: Local<Option<AppState>>,
) {
    let current = app_state.get().clone();
    if last_state.as_ref() != Some(&current) {
        if last_state.is_some() {
            fade.alpha = 1.0;
        }
        *last_state = Some(current);
    }

    if fade.alpha <= 0.0 {
        return;
    }

    let alpha = (fade.alpha * 255.0) as u8;
    let ctx = contexts.ctx_mut();
    egui::Area::new("transition_fade".into())
        .anchor(egui::Align2::LEFT_TOP, egui::vec2(0.0, 0.0))
        .order(egui::Order::Foreground)
        .interactable(false)
        .show(ctx, |ui| {
            let screen = ui.ctx().screen_rect();
            ui.painter()
                .rect_filled(screen, 0.0_f32, egui::Color32::from_black_alpha(alpha));
        });
    fade.alpha = (fade.alpha - time.delta_seconds() * 3.0).max(0.0);
}
