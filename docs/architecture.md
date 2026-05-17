# Architecture

## Overview

MindIsland is a Tauri 2 desktop app (Rust backend + React frontend) that monitors AI coding agents via system tray.

```
┌─────────────────────────────────────────────┐
│              React Frontend                  │
│  App.tsx → SessionList → SessionRow          │
│  Zustand store, Tauri IPC (invoke/listen)    │
└──────────────────┬──────────────────────────┘
                   │ Tauri IPC
┌──────────────────┴──────────────────────────┐
│              Rust Backend                    │
│                                              │
│  lib.rs          — Tauri setup + commands    │
│  tray.rs         — System tray + panel       │
│  session.rs      — SessionManager (state)    │
│  event.rs        — AgentEvent + AgentSession │
│  session_registry.rs — JSON persistence      │
│  hook_installer.rs   — settings.json mgmt    │
│                                              │
│  agents/                                     │
│    claude.rs          — Hook IPC adapter     │
│    claude_discovery.rs — JSONL transcript     │
│    opencode.rs        — SSE adapter (TODO)   │
│    ultrawork.rs       — SSE adapter (TODO)   │
└─────────────────────────────────────────────┘

┌─────────────────────────────────────────────┐
│  hooks/mindisland-claude-hook.sh             │
│  Claude Code → stdin JSON → Unix socket      │
│  PermissionRequest: bidirectional (wait)      │
│  Other events: fire-and-forget               │
└─────────────────────────────────────────────┘
```

## Data Flow

### Claude Code Events (Hook-based)

```
Claude Code hook fires
  → hooks/mindisland-claude-hook.sh reads stdin
  → Writes payload to /tmp/mindisland-claude.sock
  → claude.rs UnixListener accepts connection
  → Parses ClaudeHookPayload → AgentEvent
  → SessionManager updates HashMap<String, AgentSession>
  → Emits "sessions-updated" to frontend
  → Frontend re-renders SessionList
  → resize_panel adjusts window height
```

### Permission Flow (Bidirectional)

```
PermissionRequest hook
  → Hook script keeps socket open (24h timeout)
  → claude.rs creates oneshot channel, stores in pending map
  → SessionManager emits WaitingForApproval phase
  → Panel auto-pops up + Glass sound plays
  → User clicks Allow/Deny in frontend
  → Frontend invokes resolve_permission command
  → SessionManager calls claude.resolve_permission()
  → oneshot channel sends PermissionResponse
  → claude.rs writes JSON response to socket
  → Hook script reads response, writes to stdout
  → Claude Code reads stdout and proceeds
```

### Session Discovery (Startup)

```
App launches
  → SessionRegistry loads persisted sessions from JSON
  → ClaudeTranscriptDiscovery scans ~/.claude/projects/*.jsonl
  → Parses last 30min transcripts for session metadata
  → Creates SessionStarted + ActivityUpdated events
  → Sessions appear in panel
```

## Key Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| IPC protocol | NDJSON over Unix socket | Simple, compatible with Open Vibe Island |
| Session state | HashMap in Mutex | Simple, sufficient for <100 sessions |
| Frontend refresh | emit + panel-opened + 2s poll | Triple redundancy for reliability |
| Panel sizing | DOM scrollHeight measurement | Adapts to any content automatically |
| Hook script | Bash + Python3 | Available on all macOS, handles JSON safely |
| Notification dedup | Track last_attention_session | Prevents sound spam on polling |
| Process monitor | pgrep every 15s | Detects crashed Claude Code |

## File Paths

| Purpose | Path |
|---------|------|
| Claude Code hooks | `~/.claude/settings.json` |
| Hook script | `hooks/mindisland-claude-hook.sh` |
| IPC socket | `/tmp/mindisland-claude.sock` |
| Session registry | `~/Library/Application Support/mindisland/session-registry.json` |
| Transcripts | `~/.claude/projects/**/*.jsonl` |
| Tray icons | `src-tauri/icons/tray-{idle,active,attention}.png` |
