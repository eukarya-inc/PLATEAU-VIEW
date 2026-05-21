# syntax=docker/dockerfile:1.7
# =============================================================================
# Build stage — Rust compile with maplibre-native (Vulkan backend) deps
# Ubuntu 24.04 ships glibc 2.39, which the maplibre-native prebuilts need.
# =============================================================================
FROM ubuntu:24.04 AS builder

WORKDIR /app

# maplibre_native's build.rs uses `git clone --revision` (needs Git 2.49+),
# but Ubuntu 24.04 ships Git 2.43 — so pull a newer git from the PPA.
RUN apt-get update && apt-get install -y --no-install-recommends \
      ca-certificates gnupg software-properties-common \
    && add-apt-repository -y ppa:git-core/ppa \
    && apt-get update && apt-get install -y --no-install-recommends \
      curl git build-essential pkg-config cmake g++ \
      libssl-dev libcurl4-openssl-dev \
      libpng-dev libicu-dev libjpeg-dev libwebp-dev libuv1-dev \
      libvulkan-dev \
      # maplibre-native's Linux CMake unconditionally builds *both* the
      # Vulkan and OpenGL/GLFW platform layers, regardless of which
      # backend the Rust feature picks. We feed it the full link-time
      # dep set so CMake resolves; at runtime only the Vulkan path
      # executes, so the runtime stage stays free of these libs.
      libx11-dev libglfw3-dev libgl1-mesa-dev libegl1-mesa-dev \
      # Vulkan SPIR-V shader compilation: maplibre-native's build.rs
      # link-flags `-lglslang -lSPIRV-Tools -lSPIRV-Tools-opt
      # -lglslang-default-resource-limits` but doesn't ship those libs
      # itself — Ubuntu's `glslang-dev` provides all four as static libs.
      glslang-dev \
    && rm -rf /var/lib/apt/lists/*

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
      | sh -s -- -y --default-toolchain 1.92.0
ENV PATH="/root/.cargo/bin:${PATH}"

# Cache-friendly: deps first.
COPY Cargo.toml Cargo.lock* ./
RUN mkdir src && echo "fn main() {}" > src/main.rs \
    && cargo build --release --features maplibre \
    && rm -rf src

COPY src ./src
RUN touch src/main.rs && cargo build --release --features maplibre

# =============================================================================
# Runtime — Mesa lavapipe for headless software Vulkan
# No Xvfb, no X11, no OpenGL stack — Vulkan supports surfaceless rendering
# natively, and lavapipe is Mesa's CPU Vulkan ICD. Shrinks the image from
# ~350 MB (OpenGL+Xvfb path) to ~150 MB.
# =============================================================================
FROM ubuntu:24.04

WORKDIR /app

# `mesa-vulkan-drivers` provides lavapipe (llvmpipe-based Vulkan ICD) and
# its JSON manifest under /usr/share/vulkan/icd.d/. `libvulkan1` is the
# loader that finds and dispatches to that ICD. The other libs are
# linked at load time by libmaplibre.
RUN apt-get update && apt-get install -y --no-install-recommends \
      ca-certificates \
      libvulkan1 mesa-vulkan-drivers \
      libcurl4 libuv1 libpng16-16 libicu74 libjpeg-turbo8 libwebp7 \
      # libmaplibre is link-time tied to libX11 (see builder note); even
      # though Vulkan never touches it at runtime, dlopen fails without
      # the shared lib present. ~1 MB.
      libx11-6 \
    && rm -rf /var/lib/apt/lists/* /var/cache/apt/* \
              /usr/share/doc/* /usr/share/man/* /usr/share/info/* \
    && find /usr/share/locale -mindepth 1 -maxdepth 1 \
         ! -name 'en' ! -name 'en_US' ! -name 'locale.alias' \
         -exec rm -rf {} + \
    # mesa-vulkan-drivers ships ICDs for every GPU vendor (intel, radeon,
    # nouveau, asahi, virtio, gfxstream) — ~84 MB combined. We only need
    # lavapipe (lvp) for headless software rendering, so drop the rest
    # and their ICD manifests.
    && find /usr/share/vulkan/icd.d/ -mindepth 1 ! -name 'lvp_icd.json' -delete \
    # Glob handles both amd64 (`x86_64-linux-gnu`) and arm64
    # (`aarch64-linux-gnu`) multiarch dirs without needing to detect.
    && find /usr/lib/*-linux-gnu/ -maxdepth 1 \
         -name 'libvulkan_*.so' ! -name 'libvulkan_lvp.so' -delete

# Create non-root user *before* copying the binary, so the COPY's
# --chown writes the file with the right owner in a single layer.
# Doing `useradd` then `chown -R /app` after the COPY would duplicate
# the (~70 MB) binary into a second layer.
RUN useradd -m -s /bin/bash appuser

# Copy binary from builder
COPY --from=builder --chown=appuser:appuser /app/target/release/tile /app/tile

USER appuser

# Viewer HTML assets. Placed last so HTML-only edits invalidate just this
# layer — the binary copy, apt-get installs, and user setup above stay
# cached, turning a 5-10 minute full rebuild into a ~30 second image push.
COPY --chown=appuser:appuser static /app/static

ENV PORT=8080 \
    RUST_LOG=info \
    # Lavapipe announces itself as a CPU device; tell the Vulkan loader
    # to use it explicitly. Without this, the loader may try to enumerate
    # a non-existent GPU ICD first.
    VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json

EXPOSE 8080

# No entrypoint script needed — Vulkan doesn't require an X server.
CMD ["/app/tile"]
