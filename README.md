# Rust Audio Visualizer

[![Rust Visualizer CI/CD](https://github.com/Baptiste-lg/Rust_visualizer/actions/workflows/Ci.yml/badge.svg)](https://github.com/Baptiste-lg/Rust_visualizer/actions/workflows/Ci.yml)
[![Docker Build & Push](https://github.com/Baptiste-lg/Rust_visualizer/actions/workflows/Docker.yml/badge.svg)](https://github.com/Baptiste-lg/Rust_visualizer/actions/workflows/Docker.yml)
[![Live Demo](https://img.shields.io/badge/demo-Live%20Demo-brightgreen)](https://Baptiste-lg.github.io/Rust_visualizer/)
[![Documentation](https://img.shields.io/badge/docs-GitHub%20Pages-blue)](https://Baptiste-lg.github.io/Rust_visualizer/)

A real-time audio visualizer built with **Rust** and the **Bevy** game engine. It transforms audio from a file or microphone into reactive visual animations. Runs natively on desktop and in the browser via WebAssembly.

**[Try it live in your browser](https://Baptiste-lg.github.io/Rust_visualizer/)**

---

## Features

### Visualization Modes

| Mode | Description |
|------|-------------|
| **2D Bars** | Classic frequency spectrum analyzer with color-interpolated vertical bars |
| **3D Cubes** | 3D voxel grid with emissive glow, spread effect, and bloom |
| **3D Orb** | Deformable sphere driven by Perlin noise and bass frequencies |
| **Disc** | Shader-based concentric rings with animated sweep, bass-reactive radius |
| **Ico** | Raymarched metallic icosahedron with procedural holes, spikes, and soft shadows |
| **Waveform** | Real-time oscilloscope rendering of raw audio samples |
| **Particles** | Beat-triggered particle explosions with gravity and fade-out |

### Audio

- **FFT Analysis**: 4096-sample FFT with Hann windowing, 20Hz-20kHz range
- **Beat Detection**: Adaptive threshold on spectral flux with BPM estimation
- **Frequency Bands**: Configurable logarithmic band splitting (bass/mid/treble)
- **Audio Sources**: Load MP3/WAV files or capture microphone input
- **Playback Controls**: Play, pause, seek, and speed adjustment (0.25x-2.0x)

### Interface

- **Real-time UI** built with `bevy_egui` — tweak every parameter live
- **Preset System**: Built-in presets (Chill, Energetic, Neon, Monochrome) + JSON export/import
- **Dynamic Background**: Configurable color with optional bass-reactive pulse
- **Beat Flash**: Visual flash overlay synchronized with detected beats
- **FPS Counter**: Real-time performance monitoring
- **Interactive Camera**: Pan-orbit for 3D scenes, zoom for 2D scenes

### Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `H` | Toggle UI visibility |
| `F` | Toggle fullscreen |
| `1`-`7` | Switch visualization mode |

### Platform Support

| Platform | Audio | Rendering |
|----------|-------|-----------|
| **Linux** | ALSA/PulseAudio via cpal + rodio | Native GPU |
| **Windows** | WASAPI via cpal + rodio | Native GPU |
| **macOS** | CoreAudio via cpal + rodio | Native GPU |
| **Web (WASM)** | Web Audio API + getUserMedia | WebGPU/WebGL |

---

## Getting Started

### Prerequisites

- **Rust** toolchain — install from [rustup.rs](https://rustup.rs/)
- System dependencies for **Bevy** — see the [Bevy Setup Guide](https://bevyengine.org/learn/book/getting-started/setup/)

### Run Natively

```bash
git clone https://github.com/Baptiste-lg/Rust_visualizer.git
cd Rust_visualizer
cargo run --release
```

### Run in Browser (WASM)

```bash
# Install Trunk
cargo install trunk

# Serve locally
trunk serve --release
```

Then open `http://localhost:8080` in your browser.

### Run with Docker

```bash
docker pull ghcr.io/baptiste-lg/rust_visualizer:latest
docker run --rm ghcr.io/baptiste-lg/rust_visualizer:latest
```

---

## Architecture

```
src/
├── main.rs          # App state machine, plugin registration, background system
├── audio.rs         # Native audio pipeline, FFT analysis, beat detection
├── audio_web.rs     # WASM audio pipeline (Web Audio API via wasm-bindgen)
├── config.rs        # VisualsConfig resource with serde serialization & presets
├── camera.rs        # 3D pan-orbit camera, 2D zoom, bloom management
├── ui.rs            # egui panels, preset selector, export/import, FPS overlay
├── viz_2d.rs        # 2D frequency bar chart
├── viz_3d.rs        # 3D cube grid with spread effect
├── viz_orb.rs       # Perlin noise deformable sphere
├── viz_disc.rs      # Disc shader material setup
├── viz_ico.rs       # Ico raymarching material setup
├── viz_waveform.rs  # Oscilloscope waveform renderer
└── viz_particles.rs # Beat-triggered particle system

assets/shaders/
├── disc_shader.wgsl # Concentric ring fragment shader
└── ico_shader.wgsl  # Raymarched icosahedron SDF shader
```

## Tech Stack

| Component | Technology |
|-----------|------------|
| Engine | [Bevy](https://bevyengine.org/) 0.13 |
| UI | [bevy_egui](https://github.com/mvlabat/bevy_egui) 0.27 |
| FFT | [spectrum-analyzer](https://crates.io/crates/spectrum-analyzer) 1.7 |
| Noise | [noise](https://crates.io/crates/noise) 0.8 |
| Native Audio | [rodio](https://crates.io/crates/rodio) + [cpal](https://crates.io/crates/cpal) |
| File Picker | [rfd](https://crates.io/crates/rfd) |
| Serialization | [serde](https://serde.rs/) + serde_json |
| WASM Bridge | [wasm-bindgen](https://rustwasm.github.io/wasm-bindgen/) |

---

## DevOps & CI/CD

- **Cross-platform CI**: Automated builds on Ubuntu, Windows, and macOS
- **Quality Gates**: `cargo fmt` and `cargo clippy` enforcement
- **Security Scanning**: Dependency auditing with `rust-audit` (DevSecOps)
- **Build Caching**: `swatinem/rust-cache` for fast incremental builds
- **WASM Demo Deployment**: Automatic Trunk build and deploy to GitHub Pages
- **Docker**: Multi-stage build with Google Distroless base image, published to GHCR

---

## License

This project is open source. See the repository for license details.
