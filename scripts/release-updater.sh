#!/bin/sh
# release-updater.sh - Polls GitHub Releases and triggers full rebuild on updates
# Updates: git pull (migrations, frontend, scripts) + docker compose build + restart
set -e

# Configuration (from environment variables)
UPDATE_INTERVAL="${UPDATE_INTERVAL:-300}"
GITHUB_REPO="${GITHUB_REPO:-powerclubglobal/pcg-cc-mcp}"
GITHUB_TOKEN="${GITHUB_TOKEN:-}"
GITHUB_BRANCH="${GITHUB_BRANCH:-main}"
COMPOSE_PROJECT="${DOCKER_COMPOSE_PROJECT:-pcg-cc-mcp}"
APP_CONTAINER="${APP_CONTAINER:-pcg-cc-mcp}"
REPO_PATH="${REPO_PATH:-/repo}"

# Paths
VERSION_STATE="/app/version/CURRENT_VERSION"
LOG_PREFIX="[UPDATER]"

# Install dependencies on first run
install_deps() {
    if ! command -v curl >/dev/null 2>&1 || ! command -v jq >/dev/null 2>&1 || ! command -v docker >/dev/null 2>&1 || ! command -v git >/dev/null 2>&1; then
        log "Installing dependencies..."
        apk add --no-cache curl jq docker-cli docker-cli-compose git
    fi
}

log() {
    echo "$LOG_PREFIX $(date '+%Y-%m-%d %H:%M:%S') $1"
}

log_error() {
    echo "$LOG_PREFIX [ERROR] $(date '+%Y-%m-%d %H:%M:%S') $1" >&2
}

# Semantic version comparison
# Returns 0 if $1 > $2, 1 otherwise
version_gt() {
    test "$(printf '%s\n' "$1" "$2" | sort -V | tail -1)" = "$1" && test "$1" != "$2"
}

check_for_updates() {
    log "Checking for updates..."

    # Build auth header if token provided
    AUTH_HEADER=""
    if [ -n "$GITHUB_TOKEN" ]; then
        AUTH_HEADER="Authorization: token $GITHUB_TOKEN"
    fi

    # Query GitHub API for latest release
    RESPONSE=$(curl -sS -w "\n%{http_code}" \
        ${AUTH_HEADER:+-H "$AUTH_HEADER"} \
        -H "Accept: application/vnd.github+json" \
        "https://api.github.com/repos/${GITHUB_REPO}/releases/latest" 2>/dev/null) || {
        log_error "Failed to query GitHub API"
        return 1
    }

    HTTP_CODE=$(echo "$RESPONSE" | tail -1)
    BODY=$(echo "$RESPONSE" | sed '$d')

    # Handle rate limiting
    if [ "$HTTP_CODE" = "403" ]; then
        log_error "Rate limited by GitHub API. Will retry later."
        return 1
    fi

    # Handle not found (no releases yet)
    if [ "$HTTP_CODE" = "404" ]; then
        log "No releases found for repository"
        return 0
    fi

    if [ "$HTTP_CODE" != "200" ]; then
        log_error "GitHub API returned HTTP $HTTP_CODE"
        return 1
    fi

    # Parse latest version
    LATEST_VERSION=$(echo "$BODY" | jq -r '.tag_name' | tr -d 'v')

    if [ -z "$LATEST_VERSION" ] || [ "$LATEST_VERSION" = "null" ]; then
        log_error "Could not parse latest version from response"
        return 1
    fi

    log "Latest release: v$LATEST_VERSION, Current: v$CURRENT_VERSION"

    # Check if already at latest
    if [ "$LATEST_VERSION" = "$CURRENT_VERSION" ]; then
        log "Already at latest version"
        return 0
    fi

    # Check if update is newer (not a downgrade)
    if ! version_gt "$LATEST_VERSION" "$CURRENT_VERSION"; then
        log "Latest version $LATEST_VERSION is not newer than $CURRENT_VERSION"
        return 0
    fi

    log "New version available: v$LATEST_VERSION"
    apply_update "$LATEST_VERSION"
}

apply_update() {
    VERSION="$1"

    log "=========================================="
    log "Applying update to v${VERSION}"
    log "=========================================="

    # Step 1: Git pull to get latest code (frontend, migrations, scripts, etc.)
    log "Step 1/3: Pulling latest code from git..."
    cd "$REPO_PATH"

    # Configure git for safe directory (needed when running as different user)
    git config --global --add safe.directory "$REPO_PATH" 2>/dev/null || true

    # Fetch and pull
    git fetch origin "$GITHUB_BRANCH" || {
        log_error "Failed to fetch from origin"
        return 1
    }

    git reset --hard "origin/$GITHUB_BRANCH" || {
        log_error "Failed to reset to origin/$GITHUB_BRANCH"
        return 1
    }

    log "Code updated to $(git rev-parse --short HEAD)"

    # Step 2: Rebuild Docker image (includes frontend build + binary download)
    log "Step 2/3: Rebuilding Docker image..."
    log "  - Frontend will be rebuilt with new code"
    log "  - Server binary will be downloaded from release"
    log "  - Migrations included in new image"

    export RELEASE_VERSION="$VERSION"

    docker compose build --no-cache app || {
        log_error "Failed to build Docker image"
        return 1
    }

    log "Docker image rebuilt successfully"

    # Step 3: Restart containers with new image
    log "Step 3/3: Restarting containers..."

    docker compose up -d app || {
        log_error "Failed to restart containers"
        return 1
    }

    # Update version tracking
    echo "$VERSION" > "$VERSION_STATE"
    CURRENT_VERSION="$VERSION"

    log "=========================================="
    log "Update complete! Now running v${VERSION}"
    log "=========================================="
}

verify_requirements() {
    # Check if Docker socket is available
    if [ ! -S /var/run/docker.sock ]; then
        log_error "Docker socket not available at /var/run/docker.sock"
        return 1
    fi

    # Check if repo is mounted
    if [ ! -d "$REPO_PATH/.git" ]; then
        log_error "Repository not mounted at $REPO_PATH"
        return 1
    fi

    # Check if docker-compose.yml exists
    if [ ! -f "$REPO_PATH/docker-compose.yml" ]; then
        log_error "docker-compose.yml not found in $REPO_PATH"
        return 1
    fi

    return 0
}

# ============================================================================
# Main
# ============================================================================

install_deps

# Ensure directories exist
mkdir -p "$(dirname "$VERSION_STATE")"

# Verify requirements
verify_requirements || {
    log_error "Requirements check failed. Exiting."
    exit 1
}

# Load current version
CURRENT_VERSION="0.0.0"
if [ -f "$VERSION_STATE" ]; then
    CURRENT_VERSION=$(cat "$VERSION_STATE" | tr -d 'v')
fi

log "=========================================="
log "Release Updater Started"
log "=========================================="
log "Repository: ${GITHUB_REPO}"
log "Branch: ${GITHUB_BRANCH}"
log "Repo path: ${REPO_PATH}"
log "Current version: v${CURRENT_VERSION}"
log "Poll interval: ${UPDATE_INTERVAL}s"
log "Auth: ${GITHUB_TOKEN:+configured}${GITHUB_TOKEN:-not configured}"
log ""
log "On update will:"
log "  1. Git pull latest code (frontend, migrations, scripts)"
log "  2. Docker compose build (rebuild image)"
log "  3. Docker compose up (restart with new image)"
log "=========================================="

# Change to repo directory for docker compose commands
cd "$REPO_PATH"

# Main polling loop
while true; do
    check_for_updates || log "Update check failed, will retry next interval"

    log "Next check in ${UPDATE_INTERVAL}s..."
    sleep "$UPDATE_INTERVAL"
done
