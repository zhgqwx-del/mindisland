# CLAUDE.md

## Project

MindIsland — cross-platform AI agent monitoring panel. Sits in the system tray, monitors Claude Code / OpenCode / UltraWork sessions in real-time. Built with Tauri 2 (Rust backend) + React 19 + Vite + Tailwind 4.

Inspired by [Vibe Island](https://vibeisland.app/) and [Open Vibe Island](https://github.com/Octane0411/open-vibe-island), but cross-platform (macOS + Windows planned).

## Architecture

- **Rust backend** (`src-tauri/src/`): Tauri setup, system tray, session state, agent adapters
- **React frontend** (`src/`): Panel UI with session list, permission approval, status display
- **Hook script** (`hooks/`): Bash script that forwards Claude Code events via Unix socket
- **Docs** (`docs/`): Architecture, integration guide, lessons learned, roadmap

Data flow: `Agent hook → Hook script (stdin→socket) → Rust BridgeServer → SessionManager → Frontend`

## Build & Run

```bash
./setup.sh          # First time: install deps
./dev.sh            # Build debug + launch
./build.sh          # Release build (.app + .dmg)
```

Right-click tray icon → Quit MindIsland.

## Key Files

- `src-tauri/src/agents/claude.rs` — Claude Code hook adapter (Unix socket, bidirectional for permissions)
- `src-tauri/src/agents/claude_discovery.rs` — JSONL transcript scanner
- `src-tauri/src/session.rs` — SessionManager (state, events, tray updates, process monitor)
- `src-tauri/src/tray.rs` — System tray, panel window, right-click menu
- `src-tauri/src/event.rs` — AgentEvent + AgentSession models
- `src-tauri/src/hook_installer.rs` — Auto-install hooks in ~/.claude/settings.json
- `src-tauri/src/session_registry.rs` — Persist sessions to JSON
- `src/App.tsx` — Main app, header, auto-resize
- `src/components/SessionRow.tsx` — Session row with permission buttons
- `src/components/SessionList.tsx` — Session list with visibility filtering
- `hooks/mindisland-claude-hook.sh` — Hook script registered in Claude Code

## Conventions

- Agent adapters live in `src-tauri/src/agents/`. Each implements event translation to `AgentEvent`.
- Hooks **fail open** — if MindIsland is not running, agents continue unaffected.
- Session visibility: only active + recently completed (<2 min). No stale session clutter.
- Panel height: auto-measured from DOM scrollHeight. Never hardcode row heights.
- UTF-8 safety: always use `.chars().count()` / `.chars().take(n)` for truncation, never byte-level slicing.
- Colors: Open Vibe Island palette — ink `#0d0d0f`, paper `#f1ead9`, running `#6ea7ff`, approval `#f4a4a4`, question `#ffd58a`, completed `#6fb982`.

## Current Status

- ✅ Claude Code: full integration (15 hook events, permission approval, session discovery, auto-install, panel auto-popup)
- 🔲 OpenCode: adapter written (`agents/opencode.rs`), not yet enabled (event types need updating)
- 🔲 UltraWork: adapter written (`agents/ultrawork.rs`), not yet enabled (event types need updating)
- 🔲 Windows: code structured for cross-platform, not yet tested

## Reference

- Open Vibe Island source: `../open-vibe-island/` (design reference, NOT a dependency)
- Research doc: `../research-analysis.md`
- Design doc: `DESIGN.md`
- Detailed docs: `docs/`
