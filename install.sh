#!/bin/bash

# OpenSpore Installation Script
# Usage: ./install.sh [options]

set -e

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
RED='\033[0;31m'
NC='\033[0m' # No Color

INSTALL_DIR="$HOME/.openspore"
BIN_DIR="/usr/local/bin"
MODE="binary" # binary | compile

# Parse Arguments
if [[ "$1" == "-help" || "$1" == "--help" ]]; then
    echo "Usage: ./install.sh [options]"
    echo ""
    echo "Options:"
    echo "  (no args)   Install using pre-compiled binary (if present)"
    echo "  -compile    Force build from source using Cargo"
    echo "  -uninstall  Remove OpenSpore from system"
    echo "  -help       Show this help message"
    echo ""
    exit 0
fi

if [[ "$1" == "-uninstall" ]]; then
    echo -e "${RED}🗑️  Uninstalling OpenSpore...${NC}"

    # Remove binary
    if [ -f "$BIN_DIR/openspore" ]; then
        echo "Removing $BIN_DIR/openspore..."
        if [ -w "$BIN_DIR" ]; then
            rm "$BIN_DIR/openspore"
        else
            sudo rm "$BIN_DIR/openspore"
        fi
    fi

    # Remove config dir (optional)
    if [ -d "$INSTALL_DIR" ]; then
        read -p "Remove $INSTALL_DIR (contains config & memory)? [y/N] " -n 1 -r
        echo ""
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            rm -rf "$INSTALL_DIR"
            echo "Removed $INSTALL_DIR"
        else
            echo "Kept $INSTALL_DIR"
        fi
    fi

    echo -e "${GREEN}✅ Uninstalled.${NC}"
    exit 0
fi

if [[ "$1" == "-compile" ]]; then
    MODE="compile"
fi

echo ""
echo -e "${CYAN}🍄 OpenSpore Installer ($MODE mode)${NC}"
echo "─────────────────────────────────"
echo ""

# Installation Logic
mkdir -p "$INSTALL_DIR"

# 1. Setup Files
if [ -d "$INSTALL_DIR/.git" ]; then
     echo -e "${YELLOW}📁 Using existing repo at $INSTALL_DIR${NC}"
else
    # Verify we are in the repo to copy files from
    if [ -f "./substrate/Cargo.toml" ]; then
        # Running from source root
        # Copy substrate content if needed, but usually we just link if running from source
        # For this script we assume running FROM the repo root
        echo -e "${YELLOW}📁 Setting up environment...${NC}"
    else
        echo -e "${RED}❌ Please run ./install.sh from the project root.${NC}"
        exit 1
    fi
fi

# 2. Install Binary
TARGET_BINARY=""

if [[ "$MODE" == "binary" ]]; then
    if [ -f "./openspore" ]; then
        echo -e "${GREEN}📦 Found pre-compiled binary.${NC}"
        TARGET_BINARY="$(pwd)/openspore"
    else
        echo -e "${YELLOW}⚠️  Pre-compiled binary './openspore' not found.${NC}"
        echo -e "${YELLOW}   Switching to compile mode...${NC}"
        MODE="compile"
    fi
fi

if [[ "$MODE" == "compile" ]]; then
    # Check for Rust
    if ! command -v cargo &> /dev/null; then
        echo -e "${RED}❌ Rust/Cargo not found.${NC}"
        echo "Install Rust first: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        exit 1
    fi

    echo -e "${YELLOW}🔨 Building release binary (this may take a minute)...${NC}"
    cargo build --release --manifest-path "./substrate/Cargo.toml"
    TARGET_BINARY="$(pwd)/substrate/target/release/openspore"
fi

# 3. Create Symlink
echo -e "${YELLOW}🔗 Linking openspore to $BIN_DIR...${NC}"
if [ -w "$BIN_DIR" ]; then
    ln -sf "$TARGET_BINARY" "$BIN_DIR/openspore"
else
    echo -e "${YELLOW}   Need sudo to write to $BIN_DIR${NC}"
    sudo ln -sf "$TARGET_BINARY" "$BIN_DIR/openspore"
fi

# 4. Workspace & Env
echo -e "${YELLOW}📁 Setting up workspace dirs...${NC}"
mkdir -p "$INSTALL_DIR/workspace/context"
mkdir -p "$INSTALL_DIR/workspace/memory"
mkdir -p "$INSTALL_DIR/skills"

# Copy/Link .env if needed
if [ ! -f "$INSTALL_DIR/.env" ]; then
    if [ -f "./.env" ]; then
        cp "./.env" "$INSTALL_DIR/.env"
        echo -e "${GREEN}📝 Copied .env${NC}"
    elif [ -f "./.env.example" ]; then
        cp "./.env.example" "$INSTALL_DIR/.env"
        echo -e "${YELLOW}📝 Created .env from template (needs editing)${NC}"
    fi
fi

# Success
VERSION=$(openspore --version 2>/dev/null || echo "unknown")
echo ""
echo -e "${GREEN}✅ Installed OpenSpore $VERSION!${NC}"
echo "   Run 'openspore start'"
