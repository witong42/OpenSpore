#!/bin/bash

# OpenSpore Installation Script
# Usage: curl -fsSL https://openspore.ai/install.sh | bash
# Or: ./install.sh

set -e

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo ""
echo -e "${CYAN}🍄 OpenSpore Installer${NC}"
echo "─────────────────────────────────"
echo ""

# Check for Rust
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}❌ Rust/Cargo not found.${NC}"
    echo "Install Rust first: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

INSTALL_DIR="$HOME/.openspore"
BIN_DIR="/usr/local/bin"

# Clone or update
if [ -d "$INSTALL_DIR/.git" ]; then
    echo -e "${YELLOW}📦 Updating existing installation...${NC}"
    cd "$INSTALL_DIR"
    git pull --quiet
else
    if [ -d "$INSTALL_DIR" ]; then
        echo -e "${YELLOW}📁 Using existing ~/.openspore directory${NC}"
    else
        echo -e "${YELLOW}📦 Installing to ~/.openspore...${NC}"
        mkdir -p "$INSTALL_DIR"
    fi
fi

cd "$INSTALL_DIR"

# Check if substrate exists (for local development vs fresh install)
if [ ! -d "$INSTALL_DIR/substrate" ]; then
    echo -e "${RED}❌ substrate/ directory not found.${NC}"
    echo "For development: ensure substrate/ exists"
    exit 1
fi

# Build release binary
echo -e "${YELLOW}🔨 Building release binary (this may take a minute)...${NC}"
cargo build --release --manifest-path "$INSTALL_DIR/substrate/Cargo.toml" 2>&1 | tail -5

# Create symlink
echo -e "${YELLOW}🔗 Installing to $BIN_DIR/openspore...${NC}"
if [ -w "$BIN_DIR" ]; then
    ln -sf "$INSTALL_DIR/substrate/target/release/openspore" "$BIN_DIR/openspore"
else
    echo -e "${YELLOW}   Need sudo to write to $BIN_DIR${NC}"
    sudo ln -sf "$INSTALL_DIR/substrate/target/release/openspore" "$BIN_DIR/openspore"
fi

# Setup workspace directories
echo -e "${YELLOW}📁 Setting up workspace...${NC}"
mkdir -p "$INSTALL_DIR/workspace/identity"
mkdir -p "$INSTALL_DIR/workspace/context"
mkdir -p "$INSTALL_DIR/workspace/memory"
mkdir -p "$INSTALL_DIR/workspace/knowledge"
mkdir -p "$INSTALL_DIR/workspace/preferences"
mkdir -p "$INSTALL_DIR/workspace/autonomy/proposals"
mkdir -p "$INSTALL_DIR/workspace/cron"
mkdir -p "$INSTALL_DIR/skills"

# Create .env if missing
if [ ! -f "$INSTALL_DIR/.env" ]; then
    if [ -f "$INSTALL_DIR/.env.example" ]; then
        cp "$INSTALL_DIR/.env.example" "$INSTALL_DIR/.env"
        echo -e "${YELLOW}📝 Created .env from template${NC}"
        echo -e "${RED}   ⚠️  Edit ~/.openspore/.env and add your API keys!${NC}"
    fi
fi

# Verify installation
if command -v openspore &> /dev/null; then
    VERSION=$(openspore --version 2>/dev/null || echo "unknown")
    echo ""
    echo -e "${GREEN}✅ OpenSpore installed successfully!${NC}"
    echo ""
    echo "   Version: $VERSION"
    echo "   Binary:  $BIN_DIR/openspore"
    echo "   Config:  $INSTALL_DIR/.env"
    echo ""
    echo -e "${CYAN}Quick Start:${NC}"
    echo "   openspore start    # Start the interactive agent"
    echo "   openspore doctor   # Run system diagnostics"
    echo "   openspore --help   # See all commands"
    echo ""

    if [ ! -f "$INSTALL_DIR/.env" ] || grep -q "YOUR_KEY_HERE" "$INSTALL_DIR/.env" 2>/dev/null; then
        echo -e "${YELLOW}⚠️  Don't forget to configure your API key:${NC}"
        echo "   nano ~/.openspore/.env"
        echo ""
    fi
else
    echo -e "${RED}❌ Installation failed - 'openspore' command not found${NC}"
    exit 1
fi
