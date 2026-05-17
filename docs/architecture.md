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
│  hook_installer.rs   — Hook/plugin mgmt      │
│                                              │
│  agents/                                     │
│    claude.rs          — Socket adapter       │
│    claude_discovery.rs — JSONL transcript     │
│    opencode.rs        — Plugin installer     │
└─────────────────────────────────────────────┘

┌─────────────────────────────────────────────┐
│  Bridge Scripts (run inside agent process)   │
│                                              │
│  hooks/mindisland-claude-hook.sh             │
│    Claude Code → stdin JSON → Unix socket    │
│                                              │
│  hooks/mindisland-opencode-plugin.js         │
│    OpenCode events → Unix socket             │
│    Permission/Question → socket + HTTP reply │
└─────────────────────────────────────────────┘
```

## Unified Data Flow

All agents use the same pattern: bridge inside agent → Unix socket → Rust adapter.

```
Agent (Claude Code / OpenCode / UltraWork)
  → Bridge (bash hook / JS plugin) runs inside agent process
  → Translates agent events to NDJSON hook format
  → Writes to /tmp/mindisland-claude.sock
  → claude.rs UnixListener accepts connection
  → Parses Payload → AgentEvent (agent_id from _source field)
  → SessionManager updates HashMap<String, AgentSession>
  → Emits "sessions-updated" to frontend
  → Frontend re-renders SessionList
  → resize_panel adjusts window height
```

### Source Identification

Events carry source info to distinguish agents:
- Claude Code: `session_id` is a UUID, no `_source` field
- OpenCode: `session_id` prefixed with `opencode-`, `_source: "opencode"`
- UltraWork: `session_id` prefixed with `ultrawork-`, `_source: "ultrawork"`

## Permission Flow (Bidirectional)

### Claude Code (Hook-based)

```
PermissionRequest hook fires
  → Hook script keeps socket open (24h timeout)
  → claude.rs creates oneshot channel, stores in pending map
  → SessionManager emits WaitingForApproval phase
  → Panel auto-pops up + Glass sound plays
  → User clicks Allow/Deny in frontend
  → Frontend invokes resolve_permission (optimistic UI update)
  → SessionManager calls claude.resolve_permission()
  → oneshot channel sends PermissionResponse
  → claude.rs writes JSON response to socket
  → Hook script reads response, writes to stdout
  → Claude Code reads stdout and proceeds
```

### OpenCode (Plugin-based)

```
permission.asked event fires in plugin
  → Plugin sends NDJSON to MindIsland socket (keeps open)
  → claude.rs receives, emits PermissionRequested
  → Panel auto-pops up + sound
  → User clicks Allow/Deny
  → MindIsland writes response on socket
  → Plugin reads response
  → Plugin POSTs to OpenCode: /permission/{id}/reply
  → OpenCode proceeds
```

## Session Discovery (Startup)

```
App launches
  → SessionRegistry loads persisted sessions from JSON
  → Marks non-terminal sessions as Completed (keep original timestamp)
  → ClaudeTranscriptDiscovery scans ~/.claude/projects/*.jsonl
  → Parses last 30min transcripts for session metadata
  → Creates SessionStarted + ActivityUpdated events
  → Only truly new sessions appear (rediscovery check)
```

## Key Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| IPC protocol | NDJSON over Unix socket | Simple, shared across all agents |
| Agent bridges | Plugin/hook inside agent | Avoids auth/port discovery issues |
| Session state | HashMap in Mutex | Simple, sufficient for <100 sessions |
| Frontend refresh | emit + panel-opened + 2s poll | Triple redundancy for reliability |
| Panel sizing | DOM scrollHeight measurement | Adapts to any content automatically |
| Hook script | Bash + Python3 | Available on all macOS, handles JSON safely |
| Notification dedup | Track last_attention_session | Prevents sound spam on polling |
| Process monitor | ps exact match every 15s | Detects crashed agents |
| Panel auto-hide | Focus loss + 1.5s grace | Prevents flicker on Accessory apps |
| Permission UI | Optimistic update | Instant feedback on button click |

## File Paths

| Purpose | Path |
|---------|------|
| Claude Code hooks config | `~/.claude/settings.json` |
| Claude Code hook script | `hooks/mindisland-claude-hook.sh` |
| OpenCode plugin (source) | `hooks/mindisland-opencode-plugin.js` |
| OpenCode plugin (installed) | `~/.config/opencode/plugins/mindisland.js` |
| IPC socket | `/tmp/mindisland-claude.sock` |
| Session registry | `~/Library/Application Support/mindisland/session-registry.json` |
| Transcripts | `~/.claude/projects/**/*.jsonl` |
| Tray icons | `src-tauri/icons/tray-{idle,active,attention}.png` |
