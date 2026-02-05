# OpenSpore (Rust)

> The next-generation, high-performance autonomous agent for macOS and Linux.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Overview
OpenSpore interacts with your OS, manages your daily workflow, and thinks autonomously. Built in Rust for speed, safety, and native system integration.

## Philosophy
- **Minimalist**: No bloated runtime. Just a fast binary.
- **Sovereign**: Your data stays local in `~/.openspore/workspace`.
- **Native**: Controls Spotify, Calendar, and Filesystem directly via OS APIs.
- **Modular**: The "Brain" (LLM) is decoupled from the "Hands" (Shell) and "Eyes" (TUI).

## Installation

```bash
# Clone the repo
git clone https://github.com/your-repo/openspore.git ~/.openspore

# Build the release binary
cd ~/.openspore
cargo build --release

# Add to PATH (via symlink)
sudo ln -sf ~/.openspore/target/release/openspore /usr/local/bin/openspore
```

## Configuration
OpenSpore looks for a `.env` file in `~/.openspore/.env`.
Required keys:
- `OPENROUTER_API_KEY`: For the brain.
- `TELEGRAM_BOT_TOKEN`: For remote notifications.

## Usage
- `openspore start` - Launch the TUI dashboard.
- `openspore auto` - Trigger the autonomy engine.
- `openspore doctor` - Run system diagnostics.

## License
MIT
