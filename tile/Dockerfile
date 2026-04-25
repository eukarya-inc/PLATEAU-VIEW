# =============================================================================
# Build stage - Rust compilation with MapLibre Native dependencies
# Using Ubuntu 24.04 for glibc 2.39 (required for maplibre-native prebuilt libs)
# =============================================================================
FROM ubuntu:24.04 AS builder

WORKDIR /app

# Install Rust and build dependencies for maplibre-native
RUN apt-get update && apt-get install -y \
    curl \
    git \
    build-essential \
    pkg-config \
    libssl-dev \
    cmake \
    g++ \
    libcurl4-openssl-dev \
    libglfw3-dev \
    libuv1-dev \
    libpng-dev \
    libicu-dev \
    libjpeg-dev \
    libwebp-dev \
    libgl1-mesa-dev \
    && rm -rf /var/lib/apt/lists/*

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain 1.92.0
ENV PATH="/root/.cargo/bin:${PATH}"

# Copy manifests
COPY Cargo.toml Cargo.lock* ./

# Create dummy main.rs to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies (this layer will be cached)
RUN cargo build --release --features maplibre && rm -rf src

# Copy source code
COPY src ./src

# Build the application with maplibre feature
RUN touch src/main.rs && cargo build --release --features maplibre

# =============================================================================
# Runtime stage - Headless rendering with Xvfb and Mesa software renderer
# Using Ubuntu 24.04 for glibc 2.39 compatibility
# =============================================================================
FROM ubuntu:24.04

WORKDIR /app

# Install runtime dependencies for headless OpenGL rendering
RUN apt-get update && apt-get install -y \
    ca-certificates \
    # X11 virtual framebuffer for headless rendering
    xvfb \
    # Mesa OpenGL software rendering (GPU-less)
    libgl1-mesa-dri \
    libgl1 \
    libegl1 \
    libgbm1 \
    # Required libraries for maplibre-native
    libcurl4 \
    libglfw3 \
    libuv1 \
    libpng16-16 \
    libicu74 \
    libjpeg-turbo8 \
    libwebp7 \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /app/target/release/tile /app/tile

# Copy entrypoint script
COPY docker-entrypoint.sh /app/docker-entrypoint.sh
RUN chmod +x /app/docker-entrypoint.sh

# Create non-root user
RUN useradd -m -s /bin/bash appuser && chown -R appuser:appuser /app
USER appuser

# Environment variables for headless rendering
ENV PORT=8080
ENV RUST_LOG=info
# Virtual display for Xvfb
ENV DISPLAY=:99
# Force software rendering (no GPU required)
ENV LIBGL_ALWAYS_SOFTWARE=1
# Use LLVMpipe software renderer
ENV GALLIUM_DRIVER=llvmpipe
# Disable GPU memory allocation
ENV MESA_GL_VERSION_OVERRIDE=3.3

EXPOSE 8080

ENTRYPOINT ["/app/docker-entrypoint.sh"]
CMD ["/app/tile"]
