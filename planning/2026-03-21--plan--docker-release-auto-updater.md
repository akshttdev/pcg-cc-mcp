# Docker Release Auto-Updater — Full Stack Auto-Deployment

**Date**: 2026-03-21
**Status**: IMPLEMENTED
**Goal**: Auto-update Docker containers when GitHub Releases publishes new versions — pulls latest code (frontend, migrations, scripts) and downloads pre-compiled Rust binary

---

## Context

Current Docker deployment compiles Rust from source (~15-22 min build time). The `build-release.yml` workflow already publishes pre-compiled binaries to GitHub Releases. This plan connects the two: Docker downloads binaries instead of compiling, and an updater container polls for new releases.

**Benefits:**
- Build time: ~15-22 min → ~3-5 min (4-7x faster)
- Automatic updates when CI publishes releases
- Consistent binaries: same binary tested in CI runs in production

---

## Implementation

### 1. `Dockerfile.release` (NEW)

Replaces Rust compilation with binary download from GitHub Releases.

**Key differences from `Dockerfile`:**
- Removes: Rust toolchain, cargo, build-essential, cmake, libopus-dev, libsodium-dev
- Keeps: Frontend build (Vite), docs build (Mintlify), all runtime dependencies
- Adds: Binary download using curl with optional GitHub token auth

**Build args:**
- `RELEASE_VERSION` — version to download (default: `latest`)
- `GITHUB_REPO` — repository (default: `powerclubglobal/pcg-cc-mcp`)
- `GITHUB_TOKEN` — auth token for private repos (optional)

**Version tracking:**
- `LABEL org.opencontainers.image.version="${RELEASE_VERSION}"`
- `/app/VERSION` file embedded in image

### 2. `scripts/release-updater.sh` (NEW)

Alpine-based polling script running in separate container.

**Logic:**
1. Install deps: `curl jq docker-cli docker-cli-compose git`
2. Load current version from `/app/version/CURRENT_VERSION`
3. Poll GitHub Releases API every `UPDATE_INTERVAL` seconds
4. Compare versions (semantic versioning)
5. If newer version detected:
   - `git pull` to get latest code (frontend, migrations, scripts)
   - `docker compose build app` to rebuild image
   - `docker compose up -d app` to restart with new image

**What gets updated:**
- Server binary (downloaded from release)
- Frontend (rebuilt from pulled source)
- Database migrations (included in new image)
- Scripts and configs (from git pull)

**Error handling:**
- Git fetch/reset with error checking
- Docker build failure detection
- GitHub API rate limiting awareness

### 3. `docker-compose.yml` Changes

**App service:**
```yaml
app:
  build:
    dockerfile: Dockerfile.release
    args:
      - RELEASE_VERSION=${RELEASE_VERSION:-latest}
      - GITHUB_REPO=${GITHUB_REPO:-powerclubglobal/pcg-cc-mcp}
      - GITHUB_TOKEN=${GITHUB_TOKEN:-}
  volumes:
    - version-data:/app/version
```

**New updater service:**
```yaml
updater:
  image: alpine:latest
  container_name: pcg-updater
  environment:
    - UPDATE_INTERVAL=${UPDATE_INTERVAL:-300}
    - GITHUB_REPO=${GITHUB_REPO:-powerclubglobal/pcg-cc-mcp}
    - GITHUB_TOKEN=${GITHUB_TOKEN:-}
    - GITHUB_BRANCH=${GITHUB_BRANCH:-main}
    - REPO_PATH=/repo
  volumes:
    - /var/run/docker.sock:/var/run/docker.sock
    - .:/repo  # Mount repo for git pull + docker compose
    - version-data:/app/version:rw
  working_dir: /repo
  command: sh /repo/scripts/release-updater.sh
```

**New volumes:**
```yaml
volumes:
  version-data:
```

### 4. `scripts/docker-entrypoint.sh` Changes

Added at startup:
- Version tracking: write `/app/VERSION` to shared volume for updater to read

### 5. `scripts/manual-update.sh` (NEW)

Helper for manual version control:
```bash
./scripts/manual-update.sh 0.0.96  # Pin to specific version
./scripts/manual-update.sh latest  # Update to latest
```

---

## Files Summary

| File | Action |
|------|--------|
| `Dockerfile.release` | **Created** — Binary download instead of Rust build |
| `scripts/release-updater.sh` | **Created** — GitHub Releases polling script |
| `scripts/manual-update.sh` | **Created** — Manual update helper |
| `docker-compose.yml` | **Modified** — Add updater service, new volumes, build args |
| `scripts/docker-entrypoint.sh` | **Modified** — Add version tracking, pending update logic |

---

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `RELEASE_VERSION` | latest | Version to deploy (for initial build) |
| `GITHUB_REPO` | powerclubglobal/pcg-cc-mcp | Repository to fetch releases from |
| `GITHUB_TOKEN` | (empty) | Auth token for private repos |
| `UPDATE_INTERVAL` | 300 | Seconds between release checks (5 min) |

---

## Update Flow

```
1. Developer pushes to main
2. build-release.yml compiles binaries, publishes GitHub Release (v0.0.97)
3. Updater container polls API, detects v0.0.97 > v0.0.96
4. Updater runs: git fetch && git reset --hard origin/main
   → Gets latest frontend, migrations, scripts
5. Updater runs: docker compose build app
   → Rebuilds image with new frontend + downloads new binary
6. Updater runs: docker compose up -d app
   → Restarts container with new image
7. Server starts with new code, new binary, new migrations
```

---

## Verification

1. **Build with specific version:**
   ```bash
   RELEASE_VERSION=0.0.96 docker compose build app
   docker compose up -d
   ```

2. **Verify version tracking:**
   ```bash
   docker exec pcg-cc-mcp cat /app/VERSION
   ```

3. **Test auto-update:**
   - Push code to main, wait for CI to publish release
   - Watch updater: `docker logs -f pcg-updater`
   - Verify app restarts with new version

4. **Test rollback:**
   ```bash
   ./scripts/manual-update.sh 0.0.95
   ```
