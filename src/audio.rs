// src/audio.rs

use crate::{config::VisualsConfig, in_any_visualization_state, VisualizationEnabled};
use bevy::prelude::*;
use spectrum_analyzer::{samples_fft_to_spectrum, scaling::divide_by_N_sqrt, FrequencyLimit};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
#[cfg(not(target_arch = "wasm32"))]
use rodio::{source::Source, Decoder, Sink};
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::mpsc::{Receiver, Sender};
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

// ---------------------------------------------------------------------------
// Native-only: Symphonia duration helper
// ---------------------------------------------------------------------------

#[cfg(not(target_arch = "wasm32"))]
fn get_duration_with_symphonia(path: &Path) -> Result<Duration, Box<dyn std::error::Error>> {
    let src = std::fs::File::open(path)?;
    let mss = symphonia::core::io::MediaSourceStream::new(Box::new(src), Default::default());

    let hint = symphonia::core::probe::Hint::new();
    let meta_opts: symphonia::core::meta::MetadataOptions = Default::default();
    let fmt_opts: symphonia::core::formats::FormatOptions = Default::default();

    let probed = symphonia::default::get_probe().format(&hint, mss, &fmt_opts, &meta_opts)?;
    let format = probed.format;

    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)
        .ok_or("No supported audio track found")?;

    let time_base = track.codec_params.time_base.ok_or("Missing time base")?;
    let n_frames = track.codec_params.n_frames.ok_or("Missing frame count")?;

    let total_time = time_base.calc_time(n_frames);

    Ok(Duration::from_secs(total_time.seconds) + Duration::from_secs_f64(total_time.frac))
}

// ---------------------------------------------------------------------------
// Native-only: ArcCursor (zero-copy Read+Seek over Arc<Vec<u8>>)
// ---------------------------------------------------------------------------

#[cfg(not(target_arch = "wasm32"))]
struct ArcCursor {
    data: Arc<Vec<u8>>,
    pos: u64,
}

#[cfg(not(target_arch = "wasm32"))]
impl ArcCursor {
    fn new(data: Arc<Vec<u8>>) -> Self {
        Self { data, pos: 0 }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl std::io::Read for ArcCursor {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let start = self.pos as usize;
        if start >= self.data.len() {
            return Ok(0);
        }
        let end = (start + buf.len()).min(self.data.len());
        let n = end - start;
        buf[..n].copy_from_slice(&self.data[start..end]);
        self.pos += n as u64;
        Ok(n)
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl std::io::Seek for ArcCursor {
    fn seek(&mut self, style: std::io::SeekFrom) -> std::io::Result<u64> {
        let new_pos = match style {
            std::io::SeekFrom::Start(n) => n as i64,
            std::io::SeekFrom::End(n) => self.data.len() as i64 + n,
            std::io::SeekFrom::Current(n) => self.pos as i64 + n,
        };
        if new_pos < 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "seek before start",
            ));
        }
        self.pos = new_pos as u64;
        Ok(self.pos)
    }
}

// ---------------------------------------------------------------------------
// Native-only: AudioDataTee (taps audio samples for analysis)
// ---------------------------------------------------------------------------

#[cfg(not(target_arch = "wasm32"))]
struct AudioDataTee<S> {
    source: S,
    sender: Sender<f32>,
}

#[cfg(not(target_arch = "wasm32"))]
impl<S> Iterator for AudioDataTee<S>
where
    S: Iterator<Item = f32>,
{
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let sample = self.source.next()?;
        self.sender.send(sample).ok();
        Some(sample)
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl<S> Source for AudioDataTee<S>
where
    S: Source<Item = f32>,
{
    fn current_frame_len(&self) -> Option<usize> {
        self.source.current_frame_len()
    }
    fn channels(&self) -> u16 {
        self.source.channels()
    }
    fn sample_rate(&self) -> u32 {
        self.source.sample_rate()
    }
    fn total_duration(&self) -> Option<Duration> {
        self.source.total_duration()
    }
}

// ---------------------------------------------------------------------------
// Plugin
// ---------------------------------------------------------------------------

pub struct AudioPlugin;

#[derive(Resource)]
pub struct AnalysisTimer(pub Timer);

#[cfg(not(target_arch = "wasm32"))]
#[derive(Resource, Clone)]
pub struct AnalysisAudioSender(pub Sender<f32>);
#[cfg(not(target_arch = "wasm32"))]
pub struct AnalysisAudioReceiver(pub Receiver<f32>);

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(AnalysisTimer(Timer::new(
            Duration::from_secs_f32(1.0 / 60.0),
            TimerMode::Repeating,
        )))
        .init_resource::<AudioSamples>()
        .init_resource::<AudioAnalysis>()
        .init_resource::<SelectedMic>()
        .init_resource::<MicAudioBuffer>()
        .init_resource::<FftBuffer>()
        .init_resource::<HannCoefficients>()
        .init_resource::<CachedBandLimits>();

        // --- Native I/O systems ---
        #[cfg(not(target_arch = "wasm32"))]
        {
            let (mic_tx, mic_rx) = std::sync::mpsc::channel::<Vec<f32>>();
            let (analysis_tx, analysis_rx) = std::sync::mpsc::channel::<f32>();

            app.insert_resource(MicAudioSender(mic_tx))
                .insert_non_send_resource(MicAudioReceiver(mic_rx))
                .insert_resource(AnalysisAudioSender(analysis_tx))
                .insert_non_send_resource(AnalysisAudioReceiver(analysis_rx))
                .add_systems(
                    Update,
                    (
                        read_mic_data_system,
                        read_analysis_data_system,
                        manage_audio_playback,
                        apply_playback_changes.after(manage_audio_playback),
                        update_playback_position.after(apply_playback_changes),
                        audio_analysis_system
                            .after(read_mic_data_system)
                            .after(read_analysis_data_system)
                            .after(manage_audio_playback)
                            .run_if(|viz_enabled: Res<VisualizationEnabled>| viz_enabled.0),
                    )
                        .run_if(in_any_visualization_state),
                );
        }

        // --- WASM I/O systems ---
        #[cfg(target_arch = "wasm32")]
        {
            use crate::audio_web;
            app.add_systems(
                Update,
                (
                    audio_web::web_manage_source,
                    audio_web::web_poll_file_loaded,
                    audio_web::web_read_audio_data,
                    audio_web::web_apply_playback,
                    audio_web::web_update_position,
                    audio_analysis_system
                        .after(audio_web::web_read_audio_data)
                        .run_if(|viz_enabled: Res<VisualizationEnabled>| viz_enabled.0),
                )
                    .run_if(in_any_visualization_state),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Shared resources
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum PlaybackStatus {
    #[default]
    Paused,
    Playing,
}

#[derive(Resource, Debug, Default)]
pub struct PlaybackInfo {
    pub status: PlaybackStatus,
    pub speed: f32,
    pub duration: Duration,
    pub seek_to: Option<f32>,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) cached_file_bytes: Option<Arc<Vec<u8>>>,
}

#[derive(Resource, Debug, Default)]
pub struct PlaybackPosition {
    pub position: Duration,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) last_update: Option<Instant>,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) position_at_last_update: Duration,
}

impl PlaybackInfo {
    pub fn reset(&mut self) {
        self.status = PlaybackStatus::Paused;
        self.speed = 1.0;
        self.duration = Duration::ZERO;
        self.seek_to = None;
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.cached_file_bytes = None;
        }
    }
}

impl PlaybackPosition {
    pub fn reset(&mut self) {
        self.position = Duration::ZERO;
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.last_update = None;
            self.position_at_last_update = Duration::ZERO;
        }
    }
}

#[derive(Resource, Debug, Clone, PartialEq, Eq, Default)]
pub enum AudioSource {
    File(PathBuf),
    Microphone,
    #[default]
    None,
}

#[derive(Resource, Default)]
pub struct SelectedAudioSource(pub AudioSource);

#[derive(Resource, Default)]
pub struct SelectedMic(pub Option<String>);

#[cfg(not(target_arch = "wasm32"))]
#[derive(Resource, Clone)]
pub struct MicAudioSender(pub Sender<Vec<f32>>);
#[cfg(not(target_arch = "wasm32"))]
pub struct MicAudioReceiver(pub Receiver<Vec<f32>>);

#[allow(dead_code)]
#[cfg(not(target_arch = "wasm32"))]
pub struct MicStream(pub Option<cpal::Stream>);

#[derive(Resource, Default)]
pub struct MicAudioBuffer(pub VecDeque<f32>);

#[derive(Resource, Default, Clone)]
pub struct AudioSamples(pub VecDeque<f32>);

#[derive(Resource)]
pub struct AudioInfo {
    pub sample_rate: u32,
}

#[derive(Resource, Default)]
pub struct AudioAnalysis {
    pub frequency_bins: Vec<f32>,
    pub bass: f32,
    pub mid: f32,
    pub treble: f32,
    pub treble_average: f32,
    pub volume: f32,
    pub flux: f32,
    pub previous_spectrum: Vec<(f32, f32)>,
    spectrum_buffer: Vec<(f32, f32)>,

    // Beat detection
    pub beat_detected: bool,
    pub bpm: f32,
    flux_history: VecDeque<f32>,
    beat_timestamps: VecDeque<f64>,
}

#[derive(Resource, Default)]
pub(crate) struct FftBuffer(Vec<f32>);

#[derive(Resource)]
pub(crate) struct HannCoefficients(Vec<f32>);

impl Default for HannCoefficients {
    fn default() -> Self {
        let fft_size = 4096;
        let coeffs: Vec<f32> = (0..fft_size)
            .map(|i| {
                0.5 * (1.0
                    - (2.0 * std::f32::consts::PI * i as f32 / (fft_size as f32 - 1.0)).cos())
            })
            .collect();
        Self(coeffs)
    }
}

#[derive(Resource, Default)]
pub(crate) struct CachedBandLimits {
    num_bands: usize,
    limits: Vec<f32>,
    bins: Vec<f32>,
}

// ---------------------------------------------------------------------------
// Native-only I/O systems
// ---------------------------------------------------------------------------

#[cfg(not(target_arch = "wasm32"))]
#[allow(clippy::too_many_arguments)]
pub fn manage_audio_playback(
    mut commands: Commands,
    selected_source: Res<SelectedAudioSource>,
    sink: NonSend<Sink>,
    mut mic_stream: NonSendMut<MicStream>,
    mic_sender: Res<MicAudioSender>,
    analysis_sender: Res<AnalysisAudioSender>,
    selected_mic: Res<SelectedMic>,
    mut audio_samples: ResMut<AudioSamples>,
    mut playback_info: ResMut<PlaybackInfo>,
    mut playback_pos: ResMut<PlaybackPosition>,
) {
    if !selected_source.is_changed() {
        return;
    }

    sink.stop();
    *mic_stream = MicStream(None);
    audio_samples.0.clear();
    playback_info.reset();
    playback_pos.reset();

    match &selected_source.0 {
        AudioSource::File(path) => {
            info!("Audio source changed. Attempting to load file: {:?}", path);

            let duration = match get_duration_with_symphonia(path) {
                Ok(d) => {
                    info!("Successfully read duration with Symphonia: {:?}", d);
                    d
                }
                Err(e) => {
                    error!(
                        "Failed to get duration with Symphonia: {}. The progress bar will be incorrect.",
                        e
                    );
                    Duration::ZERO
                }
            };

            let file_bytes = match std::fs::read(path) {
                Ok(bytes) => Arc::new(bytes),
                Err(e) => {
                    error!("Failed to read music file: {e}");
                    return;
                }
            };
            let source = match Decoder::new(ArcCursor::new(file_bytes.clone())) {
                Ok(s) => s,
                Err(e) => {
                    error!("Failed to decode audio file: {e}");
                    return;
                }
            };

            commands.insert_resource(AudioInfo {
                sample_rate: source.sample_rate(),
            });

            playback_info.duration = duration;
            playback_info.status = PlaybackStatus::Playing;
            playback_info.cached_file_bytes = Some(file_bytes);
            playback_pos.last_update = Some(Instant::now());
            playback_pos.position_at_last_update = Duration::ZERO;

            let tee_source = AudioDataTee {
                source: source.convert_samples(),
                sender: analysis_sender.0.clone(),
            };

            sink.append(tee_source);
        }
        AudioSource::Microphone => {
            info!("Starting microphone capture");
            let host = cpal::default_host();
            let device = match selected_mic
                .0
                .as_ref()
                .and_then(|name| {
                    host.input_devices()
                        .ok()?
                        .find(|d| d.name().unwrap_or_default() == *name)
                })
                .or_else(|| host.default_input_device())
            {
                Some(d) => d,
                None => {
                    error!("No audio input device found");
                    return;
                }
            };
            let config = match device.default_input_config() {
                Ok(c) => c,
                Err(e) => {
                    error!("Failed to get default input config: {e}");
                    return;
                }
            };
            info!(
                "Initializing microphone: {} with config {:?}",
                device.name().unwrap_or_default(),
                config
            );
            commands.insert_resource(AudioInfo {
                sample_rate: config.sample_rate().0,
            });
            let tx = mic_sender.0.clone();
            let stream = match device.build_input_stream(
                &config.into(),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    tx.send(data.to_vec()).ok();
                },
                |err| error!("An error occurred on the audio stream: {}", err),
                None,
            ) {
                Ok(s) => s,
                Err(e) => {
                    error!("Failed to build input stream: {e}");
                    return;
                }
            };
            if let Err(e) = stream.play() {
                error!("Failed to start audio stream: {e}");
                return;
            }
            *mic_stream = MicStream(Some(stream));
        }
        AudioSource::None => {
            info!("Stopping all audio");
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn apply_playback_changes(
    mut playback_info: ResMut<PlaybackInfo>,
    mut playback_pos: ResMut<PlaybackPosition>,
    sink: NonSend<Sink>,
    selected_source: Res<SelectedAudioSource>,
    analysis_sender: Res<AnalysisAudioSender>,
) {
    if !playback_info.is_changed() {
        return;
    }

    match playback_info.status {
        PlaybackStatus::Playing => {
            if sink.is_paused() {
                sink.play();
                playback_pos.last_update = Some(Instant::now());
                playback_pos.position_at_last_update = playback_pos.position;
            }
        }
        PlaybackStatus::Paused => {
            if !sink.is_paused() {
                sink.pause();
                if let Some(last_update) = playback_pos.last_update.take() {
                    let elapsed = last_update.elapsed().as_secs_f32() * sink.speed();
                    playback_pos.position =
                        playback_pos.position_at_last_update + Duration::from_secs_f32(elapsed);
                }
                playback_pos.last_update = None;
            }
        }
    }

    if (sink.speed() - playback_info.speed).abs() > f32::EPSILON {
        if !sink.is_paused() {
            if let Some(last_update) = playback_pos.last_update.take() {
                let elapsed = last_update.elapsed().as_secs_f32() * sink.speed();
                playback_pos.position =
                    playback_pos.position_at_last_update + Duration::from_secs_f32(elapsed);
            }
            playback_pos.last_update = Some(Instant::now());
            playback_pos.position_at_last_update = playback_pos.position;
        }
        sink.set_speed(playback_info.speed);
    }

    if let Some(seek_pos_secs) = playback_info.seek_to.take() {
        if let AudioSource::File(_) = &selected_source.0 {
            info!("Seeking to {} seconds", seek_pos_secs);
            let seek_duration = Duration::from_secs_f32(seek_pos_secs);

            let Some(file_bytes) = playback_info.cached_file_bytes.as_ref() else {
                error!("No cached file bytes available for seeking");
                return;
            };
            let source = match Decoder::new(ArcCursor::new(file_bytes.clone())) {
                Ok(s) => s,
                Err(e) => {
                    error!("Failed to decode audio for seeking: {e}");
                    return;
                }
            };

            let new_source = source.skip_duration(seek_duration).convert_samples();

            let tee_source = AudioDataTee {
                source: new_source,
                sender: analysis_sender.0.clone(),
            };

            sink.stop();
            sink.clear();
            sink.append(tee_source);

            playback_pos.position = seek_duration;
            playback_pos.position_at_last_update = seek_duration;

            if playback_info.status == PlaybackStatus::Playing {
                sink.play();
                playback_pos.last_update = Some(Instant::now());
            } else {
                sink.pause();
                playback_pos.last_update = None;
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn update_playback_position(
    mut playback_info: ResMut<PlaybackInfo>,
    mut playback_pos: ResMut<PlaybackPosition>,
    sink: NonSend<Sink>,
) {
    if playback_info.status == PlaybackStatus::Playing {
        if let Some(last_update) = playback_pos.last_update {
            let elapsed_since_update = last_update.elapsed().as_secs_f32() * sink.speed();
            let new_pos = playback_pos.position_at_last_update
                + Duration::from_secs_f32(elapsed_since_update);

            if new_pos >= playback_info.duration && playback_info.duration != Duration::ZERO {
                playback_pos.position = playback_info.duration;
                playback_info.status = PlaybackStatus::Paused;
                playback_pos.last_update = None;
            } else {
                playback_pos.position = new_pos;
            }
        }
    }
}

const MAX_AUDIO_BUFFER: usize = 4096 * 10;

#[cfg(not(target_arch = "wasm32"))]
pub fn read_analysis_data_system(
    receiver: Option<NonSend<AnalysisAudioReceiver>>,
    mut buffer: ResMut<AudioSamples>,
) {
    if let Some(receiver) = receiver {
        buffer.0.extend(receiver.0.try_iter());
        if buffer.0.len() > MAX_AUDIO_BUFFER {
            let excess = buffer.0.len() - MAX_AUDIO_BUFFER;
            buffer.0.drain(..excess);
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn read_mic_data_system(
    receiver: Option<NonSend<MicAudioReceiver>>,
    mut buffer: ResMut<MicAudioBuffer>,
) {
    if let Some(receiver) = receiver {
        for new_data in receiver.0.try_iter() {
            buffer.0.extend(new_data);
        }
        if buffer.0.len() > MAX_AUDIO_BUFFER {
            let excess = buffer.0.len() - MAX_AUDIO_BUFFER;
            buffer.0.drain(..excess);
        }
    }
}

// ---------------------------------------------------------------------------
// Shared: FFT analysis system (identical on native and WASM)
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
pub fn audio_analysis_system(
    time: Res<Time>,
    mut analysis_timer: ResMut<AnalysisTimer>,
    mut audio_analysis: ResMut<AudioAnalysis>,
    audio_info: Option<Res<AudioInfo>>,
    audio_source: Res<SelectedAudioSource>,
    mut audio_samples: ResMut<AudioSamples>,
    mut mic_buffer: ResMut<MicAudioBuffer>,
    config: Res<VisualsConfig>,
    mut fft_buffer: ResMut<FftBuffer>,
    hann_coeffs: Res<HannCoefficients>,
    mut cached_band_limits: ResMut<CachedBandLimits>,
) {
    analysis_timer.0.tick(time.delta());
    if !analysis_timer.0.just_finished() {
        return;
    }

    let Some(audio_info) = audio_info else { return };
    let fft_size = 4096;

    let has_data = match &audio_source.0 {
        AudioSource::File(_) => {
            if audio_samples.0.len() < fft_size {
                false
            } else {
                fft_buffer.0.clear();
                fft_buffer
                    .0
                    .extend(audio_samples.0.iter().copied().take(fft_size));
                let drain_amount = audio_samples.0.len().saturating_sub(fft_size / 2);
                audio_samples.0.drain(..drain_amount);
                true
            }
        }
        AudioSource::Microphone => {
            if mic_buffer.0.len() < fft_size {
                false
            } else {
                fft_buffer.0.clear();
                fft_buffer
                    .0
                    .extend(mic_buffer.0.iter().copied().take(fft_size));
                let drain_amount = mic_buffer.0.len().saturating_sub(fft_size / 2);
                mic_buffer.0.drain(..drain_amount);
                true
            }
        }
        AudioSource::None => false,
    };

    if !has_data {
        return;
    }

    for (sample, coeff) in fft_buffer.0.iter_mut().zip(hann_coeffs.0.iter()) {
        *sample *= coeff;
    }

    let spectrum = match samples_fft_to_spectrum(
        &fft_buffer.0,
        audio_info.sample_rate,
        FrequencyLimit::Range(20.0, 20000.0),
        Some(&divide_by_N_sqrt),
    ) {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to compute spectrum: {e:?}");
            return;
        }
    };

    let squared_sum = fft_buffer.0.iter().map(|s| s * s).sum::<f32>();
    audio_analysis.volume = (squared_sum / fft_buffer.0.len() as f32).sqrt();

    audio_analysis.spectrum_buffer.clear();
    audio_analysis
        .spectrum_buffer
        .extend(spectrum.data().iter().map(|(f, v)| (f.val(), v.val())));

    if !audio_analysis.previous_spectrum.is_empty()
        && audio_analysis.previous_spectrum.len() == audio_analysis.spectrum_buffer.len()
    {
        let sum_of_squared_diffs = audio_analysis
            .spectrum_buffer
            .iter()
            .zip(&audio_analysis.previous_spectrum)
            .map(|((_, cur_mag), (_, prev_mag))| (cur_mag - prev_mag).powi(2))
            .sum::<f32>();
        audio_analysis.flux = sum_of_squared_diffs.sqrt();
    } else {
        audio_analysis.flux = 0.0;
    }

    let num_bands = config.num_bands;
    if num_bands == 0 {
        return;
    }

    if cached_band_limits.num_bands != num_bands {
        let min_freq = 20.0f32;
        let max_freq = 20000.0f32;
        cached_band_limits.limits = (0..num_bands)
            .map(|i| min_freq * (max_freq / min_freq).powf((i as f32 + 1.0) / num_bands as f32))
            .collect();
        cached_band_limits.bins.resize(num_bands, 0.0);
        cached_band_limits.num_bands = num_bands;
    }

    cached_band_limits.bins.fill(0.0);
    let mut current_band = 0;
    let mut treble_val = 0.0;

    for (freq, val) in spectrum.data() {
        if current_band < num_bands - 1 && freq.val() > cached_band_limits.limits[current_band] {
            current_band += 1;
        }
        cached_band_limits.bins[current_band] += val.val();

        if freq.val() > 4000.0 {
            treble_val += val.val();
        }
    }

    let smoothing = 0.5;
    if audio_analysis.frequency_bins.len() != num_bands {
        audio_analysis.frequency_bins.resize(num_bands, 0.0);
    }

    for (i, bin_val) in cached_band_limits.bins.iter().take(num_bands).enumerate() {
        audio_analysis.frequency_bins[i] =
            audio_analysis.frequency_bins[i] * smoothing + bin_val * (1.0 - smoothing);
    }

    audio_analysis.treble_average =
        audio_analysis.treble_average * smoothing + treble_val * (1.0 - smoothing);

    let quarter = (num_bands / 4).max(1);
    let half = (num_bands / 2).max(1);
    let three_quarters = (3 * num_bands / 4).max(1);
    audio_analysis.bass = audio_analysis.frequency_bins.iter().take(quarter).sum();
    audio_analysis.mid = audio_analysis
        .frequency_bins
        .iter()
        .skip(quarter)
        .take(half)
        .sum();
    audio_analysis.treble = audio_analysis
        .frequency_bins
        .iter()
        .skip(three_quarters)
        .sum();

    // --- Beat detection (adaptive threshold on flux) ---
    let audio = &mut *audio_analysis;

    const FLUX_HISTORY_SIZE: usize = 60;
    audio.flux_history.push_back(audio.flux);
    if audio.flux_history.len() > FLUX_HISTORY_SIZE {
        audio.flux_history.pop_front();
    }

    audio.beat_detected = false;
    if audio.flux_history.len() >= 10 {
        let avg_flux: f32 =
            audio.flux_history.iter().sum::<f32>() / audio.flux_history.len() as f32;
        let threshold = avg_flux * 1.8 + 0.01;

        if audio.flux > threshold {
            let elapsed = time.elapsed_seconds_f64();
            let min_beat_interval = 0.2;
            let last_beat_ok = audio
                .beat_timestamps
                .back()
                .is_none_or(|&last| elapsed - last > min_beat_interval);

            if last_beat_ok {
                audio.beat_detected = true;
                audio.beat_timestamps.push_back(elapsed);

                const MAX_BEAT_HISTORY: usize = 32;
                if audio.beat_timestamps.len() > MAX_BEAT_HISTORY {
                    audio.beat_timestamps.pop_front();
                }

                // BPM from average interval between beats
                if audio.beat_timestamps.len() >= 4 {
                    let intervals: Vec<f64> = audio
                        .beat_timestamps
                        .iter()
                        .zip(audio.beat_timestamps.iter().skip(1))
                        .map(|(a, b)| b - a)
                        .collect();
                    let avg_interval = intervals.iter().sum::<f64>() / intervals.len() as f64;
                    if avg_interval > 0.0 {
                        let raw_bpm = (60.0 / avg_interval) as f32;
                        audio.bpm = audio.bpm * 0.7 + raw_bpm * 0.3;
                    }
                }
            }
        }
    }

    std::mem::swap(&mut audio.previous_spectrum, &mut audio.spectrum_buffer);
}
