// src/ui.rs

use crate::audio::{
    AudioAnalysis, AudioSource, PlaybackInfo, PlaybackPosition, PlaybackStatus,
    SelectedAudioSource, SelectedMic,
};
use crate::config::VisualsConfig;
use crate::{in_any_visualization_state, ActiveVisualization, AppState, VisualizationEnabled};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_egui::egui::color_picker;
use bevy_egui::{egui, EguiContexts, EguiSet};
use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
use cpal::traits::{DeviceTrait, HostTrait};

// A resource to know if the UI is shown or hidden
#[derive(Resource)]
pub struct UiVisibility {
    pub visible: bool,
    pub hint_timer: Timer,
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

        app.add_systems(
            Update,
            (toggle_ui_visibility, main_ui_layout)
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

// --- UI Toggle System ---
fn toggle_ui_visibility(keyboard: Res<ButtonInput<KeyCode>>, mut ui_viz: ResMut<UiVisibility>) {
    if keyboard.just_pressed(KeyCode::KeyH) {
        ui_viz.visible = !ui_viz.visible;
        if !ui_viz.visible {
            ui_viz.hint_timer.reset();
        }
    }
}

// --- Main UI System (Layout & Content) ---
#[allow(clippy::too_many_arguments)]
fn main_ui_layout(
    mut contexts: EguiContexts,
    mut config: ResMut<VisualsConfig>,
    mut selected_source: ResMut<SelectedAudioSource>,
    mut viz_enabled: ResMut<VisualizationEnabled>,
    mut playback_info: ResMut<PlaybackInfo>,
    playback_pos: ResMut<PlaybackPosition>,
    mut ui_visibility: ResMut<UiVisibility>,
    time: Res<Time>,
    audio_analysis: Res<AudioAnalysis>,
    app_state: Res<State<AppState>>,
    mut next_app_state: ResMut<NextState<AppState>>,
    mut active_viz: ResMut<ActiveVisualization>,
    q_windows: Query<Entity, With<PrimaryWindow>>,
) {
    if q_windows.get_single().is_err() {
        return;
    }

    let ctx = contexts.ctx_mut();

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
                        egui::Stroke::new(1.0, egui::Color32::WHITE);
                    egui::Frame::default().inner_margin(8.0).show(ui, |ui| {
                        ui.label(egui::RichText::new("Press 'H' to Show UI").size(16.0));
                    });
                });
        }
        return;
    }

    // 2. LOGIC WHEN UI IS VISIBLE (Panels)
    let current_state = app_state.get();

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

            // Visualizer Choice
            ui.label("Select Visualizer:");
            ui.horizontal_wrapped(|ui| {
                if ui
                    .selectable_label(*current_state == AppState::Visualization2D, "2D Bars")
                    .clicked()
                {
                    next_app_state.set(AppState::Visualization2D);
                    active_viz.0 = AppState::Visualization2D;
                }
                if ui
                    .selectable_label(*current_state == AppState::Visualization3D, "3D Cubes")
                    .clicked()
                {
                    next_app_state.set(AppState::Visualization3D);
                    active_viz.0 = AppState::Visualization3D;
                }
                if ui
                    .selectable_label(*current_state == AppState::VisualizationOrb, "3D Orb")
                    .clicked()
                {
                    next_app_state.set(AppState::VisualizationOrb);
                    active_viz.0 = AppState::VisualizationOrb;
                }
                if ui
                    .selectable_label(*current_state == AppState::VisualizationDisc, "Disc")
                    .clicked()
                {
                    next_app_state.set(AppState::VisualizationDisc);
                    active_viz.0 = AppState::VisualizationDisc;
                }
                if ui
                    .selectable_label(*current_state == AppState::VisualizationIco, "Ico")
                    .clicked()
                {
                    next_app_state.set(AppState::VisualizationIco);
                    active_viz.0 = AppState::VisualizationIco;
                }
            });

            ui.separator();

            // Global Toggle On/Off
            let btn_text = if viz_enabled.0 {
                "⏹ Stop Render"
            } else {
                "▶ Start Render"
            };
            if ui.button(btn_text).clicked() {
                viz_enabled.0 = !viz_enabled.0;
            }

            ui.separator();
            ui.heading("🎵 Audio Source");

            // --- Microphone button ---
            if ui.button("🎤 Microphone").clicked() {
                #[cfg(target_arch = "wasm32")]
                crate::audio_web::request_microphone();

                selected_source.0 = AudioSource::Microphone;
            }

            // --- File picker button ---
            if ui.button("📂 Load File").clicked() {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("audio", &["mp3", "wav"])
                        .pick_file()
                    {
                        selected_source.0 = AudioSource::File(path);
                    }
                }

                #[cfg(target_arch = "wasm32")]
                crate::audio_web::request_file();
            }

            // Playback Controls (If file)
            if let AudioSource::File(_) = selected_source.0 {
                ui.separator();
                ui.label("Playback:");
                ui.horizontal(|ui| {
                    let icon = if playback_info.status == PlaybackStatus::Playing {
                        "⏸"
                    } else {
                        "▶"
                    };
                    if ui.button(icon).clicked() {
                        playback_info.status = match playback_info.status {
                            PlaybackStatus::Playing => PlaybackStatus::Paused,
                            PlaybackStatus::Paused => PlaybackStatus::Playing,
                        };
                    }

                    ui.label("Speed:");
                    ui.add(egui::Slider::new(&mut playback_info.speed, 0.25..=2.0).text("x"));
                });

                // Progress Bar
                if playback_info.duration > Duration::ZERO {
                    let total = playback_info.duration.as_secs_f32();
                    let mut pos = playback_pos.position.as_secs_f32();
                    let label = format!("{:.0}s / {:.0}s", pos, total);
                    if ui
                        .add(
                            egui::Slider::new(&mut pos, 0.0..=total)
                                .show_value(false)
                                .text(label),
                        )
                        .changed()
                    {
                        playback_info.seek_to = Some(pos);
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
            }

            ui.add_space(20.0);
            ui.separator();
            ui.vertical_centered(|ui| {
                ui.label(egui::RichText::new("Press 'H' to Hide UI").weak().italics());
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
