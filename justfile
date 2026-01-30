# PCG Dashboard Build Automation
# Aligned with duck-rs quality standards

# Default recipe
default:
    @just --list

# Install dependencies and verify toolchain
install:
    rustup show
    cargo fetch
    cd frontend && pnpm install

# Format all code
fmt:
    cargo fmt --all
    cd frontend && pnpm run format

# Check formatting without changes
fmt-check:
    cargo fmt --all -- --check
    cd frontend && pnpm run format:check

# Run clippy with strict denials
lint:
    cargo clippy --all-targets --all-features -- -D warnings
    cd frontend && pnpm run lint

# Fix clippy warnings automatically
fix crate="":
    @if [ -z "{{crate}}" ]; then \
        cargo clippy --fix --allow-dirty --allow-staged; \
    else \
        cargo clippy --fix --allow-dirty --allow-staged -p {{crate}}; \
    fi

# Run all tests
test:
    cargo test --workspace --no-fail-fast
    cd frontend && pnpm run check

# Run tests for a specific crate
test-crate crate:
    cargo test -p {{crate}} --no-fail-fast

# Type generation (Rust -> TypeScript)
generate-types:
    pnpm run generate-types

# Development servers
dev:
    pnpm run dev

# Backend only
backend:
    pnpm run backend:dev

# Frontend only
frontend:
    pnpm run frontend:dev

# Database migrations
migrate:
    sqlx migrate run

# Security audit
audit:
    cargo audit
    cd frontend && pnpm audit

# Full CI check (what CI should run)
ci: fmt-check lint test audit
    @echo "All CI checks passed!"

# Clean build artifacts
clean:
    cargo clean
    cd frontend && rm -rf node_modules dist
