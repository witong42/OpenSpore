# AGENTS: The Sovereign Protocol

## 🔗 [SWARM] (Execution Model)
OpenSpore operates as a high-performance **Manager-Worker Swarm** written in Rust.
- **The Manager Spore**: The central sovereign node. Responsible for horizontal synthesis and delegation.
- **Delegation Protocol**:
    1. Delegation is performed via the **`delegate` skill**.
    2. Sub-spores are spawned for tasks requiring specialized reasoning or parallel execution.
    3. Roles: `RESEARCHER`, `EXECUTOR`, `REASONER`, `PLANNER`.
    4. Sub-spores have a standard **10-minute timeout** for termination.

## 💓 [HEARTBEAT] (Pulse)
- **Interval**: 2 hours. (rust native)
- **Diagnostics**:
    1. **Disk Check**: Monitors substrate storage availability.
    2. **Journal Check**: Verifies daily journal presence (enforced after 22:00).
    3. **Substrate Health**: Executes `openspore doctor` for automated integrity repair.
- **Autonomy**: If `AUTONOMY_ENABLED=true`, triggers the `AutonomyEngine` to generate proactive proposals in `workspace/autonomy/proposals/`.
- **Feedback**: Broadcasts system status and pulse metrics via system notifications.

## 🛡️ [GUARD] (Substrate Protection)
- **Integrity**: Avoid modifying the core substrate (`substrate/` directory, engine binaries, or core configurations) unless explicitly asked.
- **Patch Zones**: The `skills/` and `workspace/` directories are dedicated spaces where the AI is permitted to apply patches, create scripts, and manage state independently.

## 🗺️ [MAP] (Meta-Guide)
- `~/.openspore/`: Root substrate.
- `substrate/`: Core Rust engine components (cli, swarm, doctor, core, tui).
- `skills/`: Extension scripts, bash plugins, and delegated capabilities.
- `workspace/`:
    - `autonomy/`: Active state, `proposals/`, and self-maintenance data.
    - `context/`: `LOGS.md` (Self-purging raw stream) and session summaries.
    - `identity/`: `USER.md`, `SOUL.md`, and `AGENTS.md`.
    - `memory/`: Daily and session journals.
    - `knowledge/`: Synthesized facts, SOTA research, and permanent knowledge items.
    - `preferences/`: User preferences and identity anchors.

## 🛠️ [CLI] (Interface)
- `openspore start`: Initialize the TUI interface.
- `openspore stop`: Terminate all active processes.
- `openspore doctor`: Run diagnostics and auto-repair substrate health.
- `openspore heartbeat`: Manually trigger the system pulse.
- `openspore journal`: Synthesize the daily journal from context.
- `openspore auto`: Trigger the autonomy anticipation engine.
- `openspore swarm`: List active sub-spores and task status.
- `openspore cron [list|install]`: Manage system cron jobs.
- `openspore think <prompt>`: Execute a one-shot reasoning cycle.
- `openspore logs`: View recent context and system logs.

---
*OpenSpore v0.1*
