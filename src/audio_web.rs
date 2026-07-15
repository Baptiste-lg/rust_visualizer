// src/audio_web.rs
//
// WASM audio backend using a lightweight JavaScript bridge.
// The JS handles Web Audio API complexity (AudioContext, AnalyserNode,
// getUserMedia, file input), and Rust reads raw time-domain samples
// each frame to feed into the shared FFT analysis pipeline.

use crate::audio::{
    AudioInfo, AudioSamples, AudioSource, MicAudioBuffer, PlaybackInfo, PlaybackPosition,
    PlaybackStatus, SelectedAudioSource,
};
use bevy::prelude::*;
use std::path::PathBuf;
use std::time::Duration;
use wasm_bindgen::prelude::*;

// ---------------------------------------------------------------------------
// JavaScript bridge (inline)
// ---------------------------------------------------------------------------

#[wasm_bindgen(inline_js = r#"
let ctx = null;
let analyser = null;
let audioEl = null;
let mediaElSource = null;
let micStream = null;
let micSource = null;
let currentBlobUrl = null;

function ensureCtx() {
    if (!ctx) {
        ctx = new AudioContext();
        analyser = ctx.createAnalyser();
        analyser.fftSize = 8192;
        analyser.smoothingTimeConstant = 0;
    }
    if (ctx.state === 'suspended') ctx.resume();
}

function stopMic() {
    if (micSource)  { try { micSource.disconnect(); } catch(_){} micSource = null; }
    if (micStream)  { micStream.getTracks().forEach(t => t.stop()); micStream = null; }
}

function stopFile() {
    if (audioEl)        { audioEl.pause(); audioEl.src = ''; audioEl = null; }
    if (mediaElSource)  { try { mediaElSource.disconnect(); } catch(_){} mediaElSource = null; }
    if (currentBlobUrl) { URL.revokeObjectURL(currentBlobUrl); currentBlobUrl = null; }
}

export function wa_load_file_url(url) {
    stopMic(); stopFile();
    ensureCtx();
    try { analyser.disconnect(); } catch(_){}

    audioEl = new Audio();
    audioEl.src = url;
    mediaElSource = ctx.createMediaElementSource(audioEl);
    mediaElSource.connect(analyser);
    analyser.connect(ctx.destination);
    audioEl.play().catch(e => console.error('Play failed:', e));
}

export function wa_start_mic() {
    stopMic(); stopFile();
    ensureCtx();
    try { analyser.disconnect(); } catch(_){}

    navigator.mediaDevices.getUserMedia({ audio: true })
        .then(stream => {
            micStream = stream;
            micSource = ctx.createMediaStreamSource(stream);
            micSource.connect(analyser);
        })
        .catch(e => console.error('Mic access denied:', e));
}

export function wa_stop_all() { stopMic(); stopFile(); }

export function wa_get_samples(out) {
    if (analyser) analyser.getFloatTimeDomainData(out);
}

export function wa_get_sample_rate() { return ctx ? ctx.sampleRate : 44100; }
export function wa_get_duration()    { return (audioEl && isFinite(audioEl.duration)) ? audioEl.duration : 0; }
export function wa_get_current_time(){ return audioEl ? audioEl.currentTime : 0; }
export function wa_is_playing()      { return audioEl ? (!audioEl.paused && !audioEl.ended) : false; }
export function wa_set_speed(s)      { if (audioEl) audioEl.playbackRate = s; }
export function wa_seek_to(t)        { if (audioEl) audioEl.currentTime = t; }
export function wa_set_paused(p) {
    if (!audioEl) return;
    if (p && !audioEl.paused)  audioEl.pause();
    if (!p && audioEl.paused)  audioEl.play().catch(e => console.error('Play failed:', e));
}

export function wa_trigger_file_input() {
    let input = document.getElementById('_rv_audio_input');
    if (!input) {
        input = document.createElement('input');
        input.type = 'file';
        input.accept = 'audio/*';
        input.id = '_rv_audio_input';
        input.style.display = 'none';
        input.addEventListener('change', () => {
            if (input.files && input.files[0]) {
                const file = input.files[0];
                const url = URL.createObjectURL(file);
                currentBlobUrl = url;
                wa_load_file_url(url);
                window.__rv_pending_file = file.name;
            }
            input.value = '';
        });
        document.body.appendChild(input);
    }
    input.click();
}

export function wa_get_pending_file() {
    const f = window.__rv_pending_file || null;
    window.__rv_pending_file = null;
    return f;
}

export function wa_screenshot() {
    const canvas = document.getElementById('bevy-canvas') || document.querySelector('canvas');
    if (!canvas) return;
    canvas.toBlob((blob) => {
        if (!blob) return;
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = 'visualizer_screenshot.png';
        a.click();
        URL.revokeObjectURL(url);
    }, 'image/png');
}

export function wa_setup_drag_drop() {
    const canvas = document.getElementById('bevy-canvas') || document.querySelector('canvas');
    if (!canvas || canvas.__rv_drop_setup) return;
    canvas.__rv_drop_setup = true;
    canvas.addEventListener('dragover', (e) => { e.preventDefault(); });
    canvas.addEventListener('drop', (e) => {
        e.preventDefault();
        if (e.dataTransfer && e.dataTransfer.files && e.dataTransfer.files[0]) {
            const file = e.dataTransfer.files[0];
            if (file.type.startsWith('audio/')) {
                const url = URL.createObjectURL(file);
                currentBlobUrl = url;
                wa_load_file_url(url);
                window.__rv_pending_file = file.name;
            }
        }
    });
}
"#)]
extern "C" {
    fn wa_start_mic();
    fn wa_stop_all();
    fn wa_get_samples(out: &js_sys::Float32Array);
    fn wa_get_sample_rate() -> u32;
    fn wa_get_duration() -> f64;
    fn wa_get_current_time() -> f64;
    fn wa_is_playing() -> bool;
    fn wa_set_speed(speed: f32);
    fn wa_seek_to(time: f64);
    fn wa_set_paused(paused: bool);
    fn wa_trigger_file_input();
    fn wa_get_pending_file() -> JsValue;
    fn wa_setup_drag_drop();
    fn wa_screenshot();
}

// ---------------------------------------------------------------------------
// Public helpers called from ui.rs
// ---------------------------------------------------------------------------

pub fn request_file() {
    wa_trigger_file_input();
}

pub fn request_microphone() {
    wa_start_mic();
}

pub fn setup_drag_drop() {
    wa_setup_drag_drop();
}

pub fn trigger_screenshot() {
    wa_screenshot();
}

// ---------------------------------------------------------------------------
// Bevy systems (registered in AudioPlugin::build on WASM)
// ---------------------------------------------------------------------------

const ANALYSER_SIZE: u32 = 8192;

/// Reads time-domain samples from the JS AnalyserNode into the correct Bevy buffer.
pub fn web_read_audio_data(
    mut audio_samples: ResMut<AudioSamples>,
    mut mic_buffer: ResMut<MicAudioBuffer>,
    selected_source: Res<SelectedAudioSource>,
) {
    let float_buf = js_sys::Float32Array::new_with_length(ANALYSER_SIZE);
    wa_get_samples(&float_buf);
    let data: Vec<f32> = float_buf.to_vec();

    match &selected_source.0 {
        AudioSource::File(_) => {
            audio_samples.0.clear();
            audio_samples.0.extend(data);
        }
        AudioSource::Microphone => {
            mic_buffer.0.clear();
            mic_buffer.0.extend(data);
        }
        AudioSource::None => {}
    }
}

/// Polls for files loaded via the JS file picker and updates Bevy state.
pub fn web_poll_file_loaded(
    mut commands: Commands,
    mut selected_source: ResMut<SelectedAudioSource>,
    mut playback_info: ResMut<PlaybackInfo>,
    mut playback_pos: ResMut<PlaybackPosition>,
) {
    let pending = wa_get_pending_file();
    if pending.is_null() || pending.is_undefined() {
        return;
    }
    let filename = pending.as_string().unwrap_or_default();
    info!("Web audio file loaded: {}", filename);

    commands.insert_resource(AudioInfo {
        sample_rate: wa_get_sample_rate(),
    });

    selected_source.0 = AudioSource::File(PathBuf::from(filename));
    playback_info.status = PlaybackStatus::Playing;
    playback_info.speed = 1.0;
    playback_info.duration = Duration::ZERO;
    playback_info.seek_to = None;
    playback_pos.reset();
}

/// Syncs PlaybackInfo changes (pause/play, speed, seek) to the JS audio element.
pub fn web_apply_playback(
    mut playback_info: ResMut<PlaybackInfo>,
    selected_source: Res<SelectedAudioSource>,
) {
    if !playback_info.is_changed() {
        return;
    }
    if !matches!(&selected_source.0, AudioSource::File(_)) {
        return;
    }

    // Sync pause/play
    match playback_info.status {
        PlaybackStatus::Playing => wa_set_paused(false),
        PlaybackStatus::Paused => wa_set_paused(true),
    }

    // Speed
    wa_set_speed(playback_info.speed);

    // Seek
    if let Some(seek_pos) = playback_info.seek_to.take() {
        wa_seek_to(seek_pos as f64);
    }
}

/// Reads the current playback position and duration from the JS audio element.
pub fn web_update_position(
    mut playback_info: ResMut<PlaybackInfo>,
    mut playback_pos: ResMut<PlaybackPosition>,
    selected_source: Res<SelectedAudioSource>,
) {
    if !matches!(&selected_source.0, AudioSource::File(_)) {
        return;
    }

    let current = wa_get_current_time();
    let duration = wa_get_duration();

    playback_pos.position = Duration::from_secs_f64(current);

    if duration > 0.0 {
        playback_info.duration = Duration::from_secs_f64(duration);
    }

    // Detect end of playback
    if duration > 0.0 && current >= duration {
        playback_info.status = PlaybackStatus::Paused;
    }
}

/// Handles source changes (stop audio when source is set to None).
pub fn web_manage_source(mut commands: Commands, selected_source: Res<SelectedAudioSource>) {
    if !selected_source.is_changed() {
        return;
    }
    match &selected_source.0 {
        AudioSource::Microphone => {
            commands.insert_resource(AudioInfo {
                sample_rate: wa_get_sample_rate(),
            });
        }
        AudioSource::None => {
            wa_stop_all();
        }
        _ => {}
    }
}
