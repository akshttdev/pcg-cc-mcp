#!/bin/bash
# APN Core - Remote Peer Installation Script
# Run this on a remote device to connect to the Alpha Protocol Network

set -e

echo "╔══════════════════════════════════════════════════════════╗"
echo "║  Alpha Protocol Network - Peer Installation             ║"
echo "╠══════════════════════════════════════════════════════════╣"
echo "║  Connecting to: Pythia Master (dashboard.powerclubglobal.com)"
echo "║  NATS Relay:    nats://nonlocal.info:4222              ║"
echo "╚══════════════════════════════════════════════════════════╝"
echo ""

# Detect OS
if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    OS="linux"
elif [[ "$OSTYPE" == "darwin"* ]]; then
    OS="macos"
elif [[ "$OSTYPE" == "msys" ]] || [[ "$OSTYPE" == "win32" ]]; then
    OS="windows"
else
    OS="unknown"
fi

echo "→ Detected OS: $OS"
echo ""

# Install dependencies based on OS
if [ "$OS" == "linux" ]; then
    echo "→ Installing Linux dependencies..."
    sudo apt update
    sudo apt install -y \
        libwebkit2gtk-4.1-dev \
        libappindicator3-dev \
        librsvg2-dev \
        patchelf \
        curl \
        git \
        nodejs \
        npm \
        build-essential

    # Install Rust if not present
    if ! command -v cargo &> /dev/null; then
        echo "→ Installing Rust..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source $HOME/.cargo/env
    fi

elif [ "$OS" == "macos" ]; then
    echo "→ Installing macOS dependencies..."
    xcode-select --install 2>/dev/null || true

    if ! command -v brew &> /dev/null; then
        echo "→ Installing Homebrew..."
        /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
    fi

    brew install node

    if ! command -v cargo &> /dev/null; then
        echo "→ Installing Rust..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source $HOME/.cargo/env
    fi

else
    echo "⚠️  Unsupported OS. Please install manually:"
    echo "   - Node.js: https://nodejs.org"
    echo "   - Rust: https://rustup.rs"
    echo "   - System dependencies for your platform"
    exit 1
fi

echo ""
echo "→ Cloning APN Core repository..."
if [ -d "pcg-cc-mcp" ]; then
    echo "   Repository already exists, pulling latest..."
    cd pcg-cc-mcp
    git pull origin new
else
    git clone -b new https://github.com/KingBodhi/pcg-cc-mcp.git
    cd pcg-cc-mcp
fi

echo ""
echo "→ Building APN Core..."
cd apn-app

echo "   Installing npm dependencies..."
npm install

echo "   Building frontend..."
npm run build

echo ""
echo "╔══════════════════════════════════════════════════════════╗"
echo "║  ✅ Installation Complete!                              ║"
echo "╠══════════════════════════════════════════════════════════╣"
echo "║  Starting APN Core...                                   ║"
echo "║                                                          ║"
echo "║  ⚠️  IMPORTANT: Save your 12-word recovery phrase!      ║"
echo "║     It will appear when the app starts.                 ║"
echo "╚══════════════════════════════════════════════════════════╝"
echo ""

# Start APN Core
npm run tauri dev
