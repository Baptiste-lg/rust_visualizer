# Stage 1: Builder
# Using the official Rust image to compile the application.
# Specific version pinning (1.83) ensures reproducible builds.
FROM rust:1.85 AS builder

# Install system libraries required for compilation (Audio, UI, Wayland support)
RUN apt-get update && apt-get install -y \
    libasound2-dev \
    libudev-dev \
    pkg-config \
    libx11-dev \
    libxcursor-dev \
    libxi-dev \
    libxrandr-dev \
    libwayland-dev \
    libxkbcommon-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/app
COPY . .

# Build the application in release mode for maximum performance optimizations
RUN cargo build --release

# Stage 2: Runtime
# Using Google Distroless image (Debian 12 based).
# Distroless images contain only the application and its runtime dependencies.
# They lack package managers, shells, and other standard programs, reducing the attack surface significantly.
FROM gcr.io/distroless/cc-debian12

# Copy the compiled binary from the builder stage.
COPY --from=builder /usr/src/app/target/release/rust-visualizer /app/rust-visualizer

# Define the entrypoint for the container
ENTRYPOINT ["/app/rust-visualizer"]