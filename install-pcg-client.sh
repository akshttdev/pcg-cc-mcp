#!/bin/bash
# PCG Client Installer
# Installs and configures the Power Club Global decentralized compute client
# Works on Mac and Linux

set -e

echo "╔════════════════════════════════════════════════════════════╗"
echo "║     Power Club Global - APN Client Installer               ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""

# Detect OS
OS="$(uname -s)"
case "${OS}" in
    Linux*)     PLATFORM=linux;;
    Darwin*)    PLATFORM=mac;;
    *)          echo "❌ Unsupported OS: ${OS}"; exit 1;;
esac

echo "📍 Detected platform: ${PLATFORM}"
echo ""

# Check for required tools
command -v cargo >/dev/null 2>&1 || {
    echo "❌ Rust/Cargo not found. Install from https://rustup.rs/"
    exit 1
}

# Installation directory
INSTALL_DIR="${HOME}/.pcg-client"
BIN_DIR="${INSTALL_DIR}/bin"
DATA_DIR="${INSTALL_DIR}/data"

echo "📦 Installing to: ${INSTALL_DIR}"
mkdir -p "${BIN_DIR}" "${DATA_DIR}"

# Clone or update repository
if [ -d "${INSTALL_DIR}/repo" ]; then
    echo "🔄 Updating existing installation..."
    cd "${INSTALL_DIR}/repo"
    git pull origin main
else
    echo "📥 Cloning repository..."
    git clone https://github.com/KingBodhi/pcg-cc-mcp "${INSTALL_DIR}/repo"
    cd "${INSTALL_DIR}/repo"
fi

# Build binaries
echo "🔨 Building client binaries..."
cargo build --release --bin server --bin apn_node

# Copy binaries
echo "📋 Installing binaries..."
cp target/release/server "${BIN_DIR}/"
cp target/release/apn_node "${BIN_DIR}/"
chmod +x "${BIN_DIR}/server" "${BIN_DIR}/apn_node"

# Create launcher script
cat > "${BIN_DIR}/pcg-client" << 'LAUNCHER_EOF'
#!/bin/bash
# PCG Client Launcher

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
INSTALL_DIR="$(dirname "${SCRIPT_DIR}")"
DATA_DIR="${INSTALL_DIR}/data"

# Ensure data directory exists
mkdir -p "${DATA_DIR}"

# Set environment
export PCG_ASSET_DIR="${DATA_DIR}"
export AUTO_START_APN=true
export AUTO_START_COMFYUI=false

# Start the client
echo "🚀 Starting PCG Client..."
echo "📊 Dashboard will be available at: http://localhost:58297"
echo "💾 Data directory: ${DATA_DIR}"
echo ""

cd "${INSTALL_DIR}/repo"
exec "${SCRIPT_DIR}/server"
LAUNCHER_EOF

chmod +x "${BIN_DIR}/pcg-client"

# Create systemd service (Linux only)
if [ "${PLATFORM}" = "linux" ]; then
    SERVICE_FILE="${HOME}/.config/systemd/user/pcg-client.service"
    mkdir -p "${HOME}/.config/systemd/user"

    cat > "${SERVICE_FILE}" << SERVICE_EOF
[Unit]
Description=PCG Client - Decentralized Compute Network
After=network.target

[Service]
Type=simple
WorkingDirectory=${INSTALL_DIR}/repo
Environment="PCG_ASSET_DIR=${DATA_DIR}"
Environment="AUTO_START_APN=true"
Environment="AUTO_START_COMFYUI=false"
ExecStart=${BIN_DIR}/server
Restart=always
RestartSec=10

[Install]
WantedBy=default.target
SERVICE_EOF

    echo "✅ Systemd service created: ${SERVICE_FILE}"
    echo ""
    echo "To enable auto-start on boot:"
    echo "  systemctl --user enable pcg-client"
    echo "  systemctl --user start pcg-client"
    echo ""
fi

# Create launchd plist (Mac only)
if [ "${PLATFORM}" = "mac" ]; then
    PLIST_FILE="${HOME}/Library/LaunchAgents/com.powerclubglobal.client.plist"
    mkdir -p "${HOME}/Library/LaunchAgents"

    cat > "${PLIST_FILE}" << PLIST_EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.powerclubglobal.client</string>
    <key>ProgramArguments</key>
    <array>
        <string>${BIN_DIR}/server</string>
    </array>
    <key>WorkingDirectory</key>
    <string>${INSTALL_DIR}/repo</string>
    <key>EnvironmentVariables</key>
    <dict>
        <key>PCG_ASSET_DIR</key>
        <string>${DATA_DIR}</string>
        <key>AUTO_START_APN</key>
        <string>true</string>
        <key>AUTO_START_COMFYUI</key>
        <string>false</string>
    </dict>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>${DATA_DIR}/client.log</string>
    <key>StandardErrorPath</key>
    <string>${DATA_DIR}/client-error.log</string>
</dict>
</plist>
PLIST_EOF

    echo "✅ LaunchAgent created: ${PLIST_FILE}"
    echo ""
    echo "To enable auto-start on boot:"
    echo "  launchctl load ${PLIST_FILE}"
    echo ""
fi

# Add to PATH
SHELL_RC=""
if [ -f "${HOME}/.bashrc" ]; then
    SHELL_RC="${HOME}/.bashrc"
elif [ -f "${HOME}/.zshrc" ]; then
    SHELL_RC="${HOME}/.zshrc"
fi

if [ -n "${SHELL_RC}" ]; then
    if ! grep -q "PCG_CLIENT_BIN" "${SHELL_RC}"; then
        echo "" >> "${SHELL_RC}"
        echo "# PCG Client" >> "${SHELL_RC}"
        echo "export PATH=\"${BIN_DIR}:\$PATH\"  # PCG_CLIENT_BIN" >> "${SHELL_RC}"
        echo "✅ Added to PATH in ${SHELL_RC}"
        echo "   Run: source ${SHELL_RC}"
    fi
fi

# Display device wallet
IDENTITY_FILE="${HOME}/.apn/node_identity.json"
if [ -f "${IDENTITY_FILE}" ]; then
    WALLET=$(cat "${IDENTITY_FILE}" | grep -o '"wallet_address":"[^"]*"' | cut -d'"' -f4)
    NODE_ID=$(cat "${IDENTITY_FILE}" | grep -o '"node_id":"[^"]*"' | cut -d'"' -f4)
    echo ""
    echo "════════════════════════════════════════════════════════════"
    echo "💰 DEVICE WALLET (for VIBE rewards)"
    echo "════════════════════════════════════════════════════════════"
    echo "  Node ID: ${NODE_ID}"
    echo "  Wallet:  ${WALLET}"
    echo ""
    echo "⚠️  This wallet is for THIS DEVICE's rewards."
    echo "   User wallets are separate and managed in the dashboard."
    echo "════════════════════════════════════════════════════════════"
fi

echo ""
echo "╔════════════════════════════════════════════════════════════╗"
echo "║  ✅ Installation Complete!                                  ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""
echo "🚀 To start the client:"
echo "   ${BIN_DIR}/pcg-client"
echo ""
echo "   or simply:"
echo "   pcg-client  (after reloading your shell)"
echo ""
echo "📊 Dashboard: http://localhost:58297"
echo "📁 Data: ${DATA_DIR}"
echo "🔑 Device Identity: ~/.apn/node_identity.json"
echo ""
echo "For support: https://powerclubglobal.com"
echo ""
