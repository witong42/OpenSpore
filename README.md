<div align="center">

# OpenSpore v1.1.3

**The Autonomous AI Agent Ecosystem**
*A minimalist Rust implementation of the OpenClaw architecture.*

[![Rust](https://img.shields.io/badge/built_with-Rust-dca282.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Version](https://img.shields.io/badge/version-1.1.3-green.svg)]()

*Architectural Sovereignty • System Integrity • Recursive Intelligence*

> [!CAUTION]
> **OpenSpore is a powerful, autonomous AI agent with full read/write access to your system.** It is capable of executing shell commands, modifying files, and performing complex actions without manual approval for every step. Use it in a secure, isolated environment if possible, and monitor its activity closely.

</div>

---

## 📖 Overview

**OpenSpore** is a high-performance, autonomous AI agent engine built in Rust. It serves as a **minimalist, simplified implementation of the OpenClaw architecture**, focusing on core autonomy, safety, and parallel tool execution without the overhead of larger frameworks.

It is designed to be a "living" system that operates continuously, managing its own memory, executing complex multi-step tasks, and orchestrating a **parallel swarm of specialized sub-agents**. It features a rich Terminal User Interface (TUI) for observation, and a robust "Brain" that interfaces with powerful LLMs (Anthropic Claude, Google Gemini, OpenAI GPT) via **Parallel Tool Execution**.

This release, **v1.1.3**, signals the transition from individual autonomy to **Swarm Intelligence**. It introduces hierarchical task decomposition, a process-wide concurrency limit of 6 simultaneous spores, and a negotiation-based consensus loop for autonomous proposals.

## 🏗 Architecture

OpenSpore is composed of several independent but interconnected crates within a workspace:

### 1. **Crates (Core Engine)**
The foundational layer providing configuration, state management, and the event bus. It ensures stability and high performance.

### 2. **Brain**
The cognitive center. It handles:
- **LLM Interface:** Connects to AI models via unified APIs.
- **Chain-of-Thought (CoT):** A recursive reasoning engine where agents explain their logic before acting.
- **Parallel Tool Execution:** The ability to execute multiple tools (including delegation) simultaneously in a single turn.

### 3. **Swarm**
The autonomous orchestration system.
- **Hierarchical Task Decomposition:** The `AutonomyEngine` acts as a **Planner**, breaking complex goals into specialized `AtomicTasks`.
- **Negotiation & Consensus:** Prototypical "Reviewer" spores audit proposals to ensure safety and value through a consensus loop.
- **Parallel Delegation:** Support for up to **6 simultaneous sub-spores** with unified **concurrency control** and a 3-minute timeout.

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

OpenSpore must be built from source to ensure binary compatibility and system integrity.

### Prerequisites

| Tool | Purpose | Minimum Version |
| :--- | :--- | :--- |
| **Rust / Cargo** | Building the engine | 1.70+ |
| **Python 3** | Running specific skills | 3.10+ |
| **Node.js** | Running JS-based skills | 18+ |
| **Git** | Memory & Engine updates | Latest |

**System Dependencies (Linux/Ubuntu):**
```bash
sudo apt update && sudo apt install -y pkg-config libssl-dev
```

### Quick Install

Use the provided installer script to build and link the binary:

```bash
# Build from source and install
./install.sh

# Uninstall OpenSpore
./install.sh -uninstall
```

---

## ⚙️ Configuration

OpenSpore requires a `.env` file in the project root (`~/.openspore/.env` by default).

**Note:** Currently, OpenSpore **only supports OpenRouter** for LLM connectivity to access various models (Claude, Gemini, GPT-4) via a unified interface.

### 1. Setup Environment
Run the interactive doctor to guide you through initial `.env` setup:

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

# Security & Stability
SAFE_MODE_ENABLED=true       # Restrict AI from modifying its own logic (crates)
```

---

## 🛡️ Security & Safe Mode

Because OpenSpore is an autonomous agent with the ability to modify files and run shell commands, it includes a **Safe Mode** to protect the integrity of the core system (the crates).

When `SAFE_MODE_ENABLED=true` is set in your `.env`:

1. **Write Protection**: The AI is blocked from modifying files inside the `crates/` directory.
2. **Safe Zones**: The AI **is permitted** to modify files in `skills/` and `workspace/`. This allows for new capabilities and state management while keeping the engine logic isolated.
3. **Config Protection**: Key system files like `.env`, `Cargo.toml`, and `install.sh` are read-only for the agent.
4. **Command Filtering**: Dangerous shell commands (e.g., `rm`, `mv`, `sed`) are filtered and blocked if they target core crates or config.

We recommend keeping Safe Mode **enabled** unless you are specifically instructing the agent to perform an authorized core system upgrade.
### 2. Define Identity
OpenSpore's "recursive intelligence" is shaped by Markdown files in `~/.openspore/workspace/identity/`.

- **`SOUL.md`**: Define your agent's core personality, tone, and ethical boundaries.
- **`USER.md`**: Provide context about yourself, your projects, and your preferences so the Brain can better assist you.
- **`AGENTS.md`**: Define agent roles and capabilities.

### 3. Telegram Integration (Optional)
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
- **`openspore doctor`**: Self-diagnosis tool to verify API keys, dependencies, and engine health.
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
<i>"The Spore grows. The Engine endures."</i>
</div>
