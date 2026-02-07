<div align="center">

# OpenSpore v1.0.1

**The Autonomous AI Agent Substrate**

[![Rust](https://img.shields.io/badge/built_with-Rust-dca282.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Version](https://img.shields.io/badge/version-1.0.1-green.svg)]()

*Architectural Sovereignty • Substrate Integrity • Recursive Intelligence*

> [!CAUTION]
> **OpenSpore is a powerful, autonomous AI agent with full read/write access to your system.** It is capable of executing shell commands, modifying files, and performing complex actions without manual approval for every step. Use it in a secure, isolated environment if possible, and monitor its activity closely.

</div>

---

## 📖 Overview

**OpenSpore** is a high-performance, autonomous AI agent substrate built in Rust. It is designed to be a "living" system that operates continuously, managing its own memory, executing complex multi-step tasks, and creating "sub-spores" (autonomous sub-agents) to delegate work. It features a rich Terminal User Interface (TUI) for interaction and observation, and a robust "Brain" that interfaces with powerful LLMs (Anthropic Claude, Google Gemini, OpenAI GPT).

This release, **v1.0.1**, marks the first stable milestone of the OpenSpore ecosystem.

## 🏗 Architecture

OpenSpore is composed of several independent but interconnected crates within a workspace:

### 1. **Substrate (Core)**
The foundational layer providing configuration, state management, and the event bus. It ensures stability and high performance.

### 2. **Brain**
The cognitive center. It handles:
- **LLM Interface:** Connects to AI models via unified APIs.
- **Thinking Process:** A recursive, stream-of-thought engine that allows the agent to "reason" before acting.
- **Context Assembly:** Dynamically gathering relevant files and memory for each prompts.

### 3. **Swarm**
The agentic capability system.
- **Spore Delegation:** The ability to spawn independent sub-agents ("Spores") to handle specific tasks (e.g., "Research this library", "Audit this file").
- **Task Management:** Asynchronous execution and result synthesis.

### 4. **Memory**
A persistent context system.
- **Short-term:** Working context for current tasks.
- **Long-term:** Vector-based or file-based archival of past interactions and learnings.
- **Journaling:** Automated synthesis of daily activities.

### 5. **TUI (Terminal User Interface)**
A beautiful, highly-responsive interface built with `ratatui`.
- **Visualize Thinking:** Watch the agent's thought process unfold in real-time layers.
- **Interactive:** Full keyboard and mouse support for navigation.

---

## 🚀 Installation

### Quick Install
We provide a versatile installer script:

```bash
# default: Install pre-compiled binary (fastest)
./install.sh

# Force compile from source (requires Rust)
./install.sh -compile

# Uninstall OpenSpore
./install.sh -uninstall

# Show help
./install.sh -help
```

### Build from Source
If the pre-compiled binary doesn't work for your architecture (macOS/Linux), build from source:

```bash
cargo build --release --manifest-path ./substrate/Cargo.toml
```

---

## ⚙️ Configuration

OpenSpore requires a `.env` file in the project root (`~/.openspore/.env` by default).

**Note:** Currently, OpenSpore **only supports OpenRouter** for LLM connectivity to access various models (Claude, Gemini, GPT-4) via a unified interface.

### 1. Setup Environment
Run the interactive doctor to guide you through setup:
```bash
openspore doctor
```

### 2. Telegram Integration (Optional)
To control OpenSpore remotely via Telegram:

1.  **Create a Bot:**
    *   Open Telegram and search for **@BotFather**.
    *   Send `/newbot` and follow instructions.
    *   Copy the **HTTP API Token** provided.

2.  **Get your Chat ID:**
    *   Search for **@userinfobot** (or any "Get ID" bot).
    *   Click "Start" to see your numerical ID (e.g., `123456789`).

3.  **Update `.env`:**
    ```env
    TELEGRAM_BOT_TOKEN=your_token_here
    TELEGRAM_ALLOWED_USERS=your_id_here
    ```

**Key `.env` Variables:**
```env
# AI Provider (OpenRouter Only)
OPENROUTER_API_KEY=sk-or-...
OPENROUTER_MODEL=google/gemini-2.0-flash-001  # Default model

# Search
BRAVE_SEARCH_API_KEY=...    # For web search capability

# System
OPENSPORE_ROOT=.openspore

# Autonomy
AUTONOMY_ENABLED=true       # Enable/Disable background agent
```

---

## 🎮 Usage

### TUI Mode (Default)
Start the interactive agent interface:
```bash
openspore start
```

**Shortcuts:**
- `Up` / `Down`: Jump between message layers (Headers).
- `Shift + Up` / `Shift + Down`: Fast jump (5 items).
- `Mouse Scroll`: Smooth scroll through content.
- `Space`: Toggle fold/unfold of thought layers.
- `Enter`: Submit message.
- `Shift + Enter` (or `Alt + Enter`): Multi-line input (New line).
- `§` (Paragraph Section Key): Toggle Mouse Capture (useful for copy-pasting from terminal).
- `Esc`: Quit.

### CLI Commands
OpenSpore provides a powerful CLI for management and automation.

- **`openspore start`**: Launches the primary TUI interface.
- **`openspore stop`**: Terminates all running OpenSpore background processes.
- **`openspore doctor`**: Self-diagnosis tool to verify API keys, dependencies, and environment health.
- **`openspore cron [list|install]`**: Manage the system's autonomous schedules (install creates actual system crontabs).
- **`openspore job <name>`**: Manually execute a specific job defined in the workspace cron registry.
- **`openspore auto`**: Triggers the **Autonomy Engine** to analyze recent context and propose new tasks.
- **`openspore swarm`**: Discovers and lists all active sub-spores currently executing delegated tasks.
- **`openspore think "<prompt>"`**: Executes a single thinking cycle and returns the result (Markdown).
- **`openspore logs`**: Quickly view the most recent context and thinking logs.
- **`openspore heartbeat`**: Performs a system status check and triggers autonomy if necessary.
- **`openspore journal`**: Synthesizes the last 24 hours of activity into a structured daily report.

---

## 📁 Workspace Structure

OpenSpore stores all its persistent data in `~/.openspore/workspace`.

| Folder | Description |
| :--- | :--- |
| `autonomy/` | Contains task proposals and the state of background autonomous actions. |
| `context/` | Stores active task logs and "short-term" window memory for the Brain. |
| `cron/` | Registry of scripts and schedules for autonomous background tasks. |
| `identity/` | Defines the agent's core personality, user profile, and system roles. |
| `knowledge/` | Distilled "long-term" knowledge items and research archives. |
| `memory/` | Persistent vector stores and interaction history indexing. |
| `preferences/` | User-defined settings for UI, models, and behavior overrides. |

---

## 🧠 Skills System

Skills are the "tools" the agent can use. They are defined in the `skills/` directory.
Each skill usually consists of a `SKILL.md` (definition) and associated scripts.

**To add a new skill:**
1. Create a directory in `skills/my_new_skill`.
2. The agent will automatically discover it.

---

<div align="center">
<i>"The Spore grows. The Substrate endures."</i>
</div>
