# Build stage — use Debian for glibc (Alpine musl can't link native C deps like opus/sodium)
FROM node:24-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    curl \
    build-essential \
    perl \
    python3 \
    python3-pip \
    cmake \
    pkg-config \
    libssl-dev \
    libopus-dev \
    libsodium-dev \
    && rm -rf /var/lib/apt/lists/*

# Install Rust (nightly toolchain matching rust-toolchain.toml)
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain nightly-2025-05-18
ENV PATH="/root/.cargo/bin:${PATH}"
# Disable sccache in Docker (not installed, not needed for one-shot builds)
ENV RUSTC_WRAPPER=""

# Set working directory
WORKDIR /app

# Copy package files for dependency caching
COPY package*.json pnpm-lock.yaml pnpm-workspace.yaml ./
COPY frontend/package*.json ./frontend/
COPY npx-cli/package*.json ./npx-cli/

# Install pnpm and dependencies
RUN npm install -g pnpm && pnpm install

# Copy source code
COPY . .

# Build application
ENV SQLX_OFFLINE=true
# Remove sccache wrapper from cargo config (not installed in Docker)
RUN sed -i '/rustc-wrapper/d' .cargo/config.toml || true
RUN npm run generate-types
RUN cd frontend && npm install --ignore-scripts --legacy-peer-deps && npx vite build --mode production
RUN cargo build --release --bin server

# Build Mintlify documentation as static site
RUN mkdir -p /app/.docs-output && \
    (cd docs && npx mintlify build --output /app/.docs-output || \
     echo '<html><head><meta http-equiv="refresh" content="0;url=/"></head><body>Docs not built. Redirecting...</body></html>' > /app/.docs-output/index.html)

# Runtime stage - Use CUDA-enabled base for GPU support
FROM nvidia/cuda:12.1.0-runtime-ubuntu22.04 AS runtime

# Set timezone non-interactively to avoid tzdata prompts
ENV DEBIAN_FRONTEND=noninteractive
ENV TZ=UTC

# Install runtime dependencies including Python 3.11 for Chatterbox
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    tini \
    wget \
    curl \
    sqlite3 \
    software-properties-common \
    git \
    zstd \
    && add-apt-repository ppa:deadsnakes/ppa \
    && apt-get update \
    && apt-get install -y --no-install-recommends \
    python3.11 \
    python3.11-dev \
    python3.11-venv \
    python3-pip \
    && rm -rf /var/lib/apt/lists/*

# Set Python 3.11 as default
RUN update-alternatives --install /usr/bin/python3 python3 /usr/bin/python3.11 1 \
    && update-alternatives --install /usr/bin/python python /usr/bin/python3.11 1

# Install Ollama for local LLM inference
RUN curl -fsSL https://ollama.ai/install.sh | sh

# Create app user for security
RUN groupadd -g 1001 appgroup && \
    useradd -u 1001 -g appgroup -m -s /bin/bash appuser

# Upgrade pip, setuptools, and wheel for Python 3.11
RUN python3.11 -m pip install --upgrade pip setuptools wheel

# Install Chatterbox TTS via pip (simpler and more reliable)
RUN python3.11 -m pip install --no-cache-dir chatterbox-tts

# Copy binary and frontend assets from builder
COPY --from=builder /app/target/release/server /usr/local/bin/server
# Copy frontend to build directory (source for volume sync)
COPY --from=builder /app/frontend/dist /app/frontend-build
# Also copy to dist for initial volume population
COPY --from=builder /app/frontend/dist /app/frontend/dist

# Copy built docs (if available)
# Copy to build directory (source for volume sync)
COPY --from=builder /app/.docs-output /app/docs-build
# Also copy to docs-site for initial volume population
COPY --from=builder /app/.docs-output /app/docs-site

# Copy Python scripts for Chatterbox server
COPY scripts/chatterbox_server.py /app/scripts/chatterbox_server.py
RUN chmod +x /app/scripts/chatterbox_server.py

# Copy dev_assets as SEED (not to final location) - preserves existing data
COPY --from=builder /app/dev_assets /app/dev_assets_seed

# Copy entrypoint script
COPY scripts/docker-entrypoint.sh /usr/local/bin/docker-entrypoint.sh
RUN chmod +x /usr/local/bin/docker-entrypoint.sh

# Create necessary directories and set permissions
RUN mkdir -p /repos /app/dev_assets /app/backups /app/scripts /root/.ollama \
    && chown -R appuser:appgroup /repos /app \
    && chown -R appuser:appgroup /root/.ollama

# Pre-pull Ollama models as root before switching user
# This ensures models are available when the container starts
# Using actual Ollama model names from their registry
RUN ollama serve & \
    sleep 5 && \
    ollama pull gpt-oss && \
    ollama pull deepseek-r1 && \
    pkill ollama && \
    sleep 2

# Switch to non-root user
USER appuser

# Set runtime environment
ENV HOST=0.0.0.0
ENV PORT=3001
ENV FRONTEND_PORT=3000
ENV BACKEND_PORT=3001
ENV OLLAMA_BASE_URL=http://localhost:11434
ENV CHATTERBOX_PORT=8100
ENV CHATTERBOX_DEVICE=cuda
EXPOSE 3001 11434 8100

# Set working directory
WORKDIR /app

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD wget --quiet --tries=1 --spider "http://127.0.0.1:3001/api/health" || exit 1

# Run the application with entrypoint that preserves data
ENTRYPOINT ["/usr/bin/tini", "--", "docker-entrypoint.sh"]
CMD ["server"]
