# Agent Integration Guide

## Supported Agents

| Agent | Mechanism | Status | Config Path |
|-------|-----------|--------|-------------|
| **Claude Code** | Bash Hook → Unix Socket | ✅ Active | `~/.claude/settings.json` |
| **OpenCode** | JS Plugin → Unix Socket | 🚧 Phase 2 | `~/.config/opencode/plugins/` |
| **UltraWork** | JS Plugin → Unix Socket | 🔲 Phase 3 | TBD |

## Unified Architecture

All agents use the same pattern: **bridge running inside the agent's process → Unix socket → MindIsland**.

```
Agent Process                        MindIsland
┌──────────────────┐                ┌──────────────┐
│ Claude Code      │                │              │
│  └─ bash hook ───┼──┐             │  claude.rs   │
│                  │  │  Unix       │  (listener)  │
│ OpenCode         │  ├─ socket ───►│              │
│  └─ JS plugin ──┼──┘             │  session.rs  │
│                  │  /tmp/         │  (manager)   │
│ UltraWork        │  mindisland-  │              │
│  └─ JS plugin ──┼──┘ claude.sock │  tray/panel  │
└──────────────────┘                └──────────────┘
```

All bridges share `/tmp/mindisland-claude.sock` and send events in the same NDJSON format (Claude Code hook payload format). Source is identified via `_source` field and/or session_id prefix.

### Why Plugin Mode (Not SSE Direct Connect)

OpenCode and UltraWork run HTTP servers with SSE endpoints, but directly connecting from MindIsland is problematic:
- Server port is dynamic (Electron sidecar assigns random ports)
- Authentication credentials are only available inside the agent process
- Permission/question replies require HTTP POST back to the agent, needing auth headers

The plugin approach solves all of these — the plugin runs inside the agent process with full access to the server context.

## Claude Code Integration

### Hook Events (14 types)

| Event | Phase | Description |
|-------|-------|-------------|
| `SessionStart` | Completed | New session created |
| `UserPromptSubmit` | Running | User sent a message |
| `PreToolUse` | Running | About to call a tool |
| `PostToolUse` | Running | Tool call completed |
| `PostToolUseFailure` | Running | Tool call failed |
| `PermissionRequest` | WaitingForApproval | Needs user approval (bidirectional) |
| `PermissionDenied` | Completed | Permission was denied |
| `Notification` | Running | Informational notification |
| `PreCompact` | Running | About to compact context |
| `Stop` | Completed | Turn finished |
| `StopFailure` | Completed | Turn failed |
| `SessionEnd` | (removed) | Session terminated |
| `SubagentStart` | Running | Subagent spawned |
| `SubagentStop` | Running | Subagent finished |

### Hook Payload Format (JSON, snake_case)

```json
{
  "session_id": "97ebc876-9e17-4181-ab25-76ea43bac08e",
  "hook_event_name": "PreToolUse",
  "cwd": "/Users/user/project",
  "tool_name": "Bash",
  "tool_input": {"command": "npm install"},
  "model": "claude-opus-4-6",
  "last_assistant_message": "I'll install the dependencies...",
  "transcript_path": "~/.claude/projects/.../session.jsonl"
}
```

### Permission Response Format (stdout JSON)

```json
{
  "continue": true,
  "suppressOutput": true,
  "hookSpecificOutput": {
    "hookEventName": "PermissionRequest",
    "decision": {
      "behavior": "allow"
    }
  }
}
```

### Hook Script Details

Location: `hooks/mindisland-claude-hook.sh`

- **Non-permission events**: Read stdin → write to socket → exit (fire-and-forget)
- **PermissionRequest**: Read stdin → write to socket → keep connection open → wait for response → write to stdout
- **Fail-open**: If MindIsland not running, exits silently — Claude Code unaffected
- **Retry**: Retries socket connection once after 0.5s (handles startup race)
- **Temp file**: Uses `mktemp` with `trap EXIT` for safe cleanup

## OpenCode Integration (Phase 2)

### Plugin Architecture

OpenCode supports JS plugins in `~/.config/opencode/plugins/`. The MindIsland plugin:

1. Receives OpenCode's event bus events (`session.created`, `message.part.updated`, `permission.asked`, etc.)
2. Maps them to Claude Code hook format (same `hook_event_name` / `session_id` / `tool_name` fields)
3. Sends NDJSON to MindIsland's Unix socket (`/tmp/mindisland-claude.sock`)
4. For permissions: sends event → waits for socket response → POSTs decision back to OpenCode API

### OpenCode Event Types → Hook Mapping

| OpenCode Event | Hook Event | Notes |
|----------------|------------|-------|
| `session.created` | `SessionStart` | session_id prefixed with `opencode-` |
| `session.status` (idle) | `Stop` | Session turn completed |
| `session.updated` (archived) | `SessionEnd` | Session removed |
| `message.part.updated` (text, user) | `UserPromptSubmit` | User message |
| `message.part.updated` (tool, running) | `PreToolUse` | Tool execution start |
| `message.part.updated` (tool, completed) | `PostToolUse` | Tool execution done |
| `permission.asked` | `PermissionRequest` | Bidirectional — needs reply |
| `question.asked` | `PermissionRequest` (AskUserQuestion) | Bidirectional — needs reply |

### Permission Reply Flow

```
OpenCode fires permission.asked event
  → Plugin sends to MindIsland socket (keeps connection open)
  → MindIsland shows panel with Allow/Deny buttons
  → User clicks → MindIsland sends response on socket
  → Plugin reads response
  → Plugin POSTs to OpenCode: POST /permission/{id}/reply
  → OpenCode proceeds
```

### Plugin File

Location: `hooks/mindisland-opencode-plugin.js`
Auto-installed to: `~/.config/opencode/plugins/mindisland.js`

### OpenCode Server Details

- Server URL: provided by OpenCode at plugin init (dynamic port)
- Auth: `OPENCODE_SERVER_USERNAME` / `OPENCODE_SERVER_PASSWORD` env vars (Basic auth)
- Permission reply: `POST /permission/{id}/reply` with `{ reply: "once"|"always"|"reject" }`
- Question reply: `POST /question/{id}/reply` with `{ answers: [...] }`
- Directory header: `x-opencode-directory` for multi-session routing

## Adding a New Agent

1. Write a bridge (hook script or JS plugin) that forwards events to `/tmp/mindisland-claude.sock`
2. Use Claude Code hook payload format with `_source` field for identification
3. Prefix session_id with agent name (e.g., `opencode-{id}`)
4. Register agent metadata in `session.rs` → `agent_meta()` function (name, color)
5. Add auto-installer logic in `hook_installer.rs` or a new installer module

### Agent Metadata

```rust
fn agent_meta(agent_id: &str) -> AgentMeta {
    match agent_id {
        "claude-code" => AgentMeta { name: "Claude Code", color: "#d97742" },
        "opencode"    => AgentMeta { name: "OpenCode",    color: "#ffb547" },
        _             => AgentMeta { name: "Agent",       color: "#71717a" },
    }
}
```
