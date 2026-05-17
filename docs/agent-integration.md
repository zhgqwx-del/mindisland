# Agent Integration Guide

## Supported Agents

| Agent | Mechanism | Status | Config Path |
|-------|-----------|--------|-------------|
| **Claude Code** | Hook + Unix Socket | ✅ Active | `~/.claude/settings.json` |
| **OpenCode** | SSE (HTTP) | 🔲 Planned | `~/.config/opencode/` |
| **UltraWork** | SSE (HTTP) | 🔲 Planned | `~/.config/ultrawork/opencode.json` |

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
- **Temp file**: Uses `mktemp` to safely pass JSON (avoids shell quoting issues)

## Adding a New Agent

1. Create `src-tauri/src/agents/new_agent.rs` implementing the adapter
2. Register in `src-tauri/src/agents/mod.rs`
3. Add to `SessionManager::start_monitoring()` in `session.rs`
4. Add agent metadata in `agent_meta()` function (name, color)

### Agent Adapter Pattern

```rust
// For SSE-based agents (OpenCode, UltraWork):
pub struct MyAgentAdapter {
    base_url: String,
    auth_header: String,
}

impl MyAgentAdapter {
    pub async fn is_available(&self) -> bool { /* health check */ }
    pub async fn start(&self, tx: mpsc::Sender<AgentEvent>) -> Result<(), String> {
        // Subscribe to SSE, translate events, send to tx
    }
}

// For Hook-based agents (Claude Code forks):
// Reuse ClaudeCodeAdapter with different socket path and hook source
```

## OpenCode / UltraWork Integration (Planned)

Both use OpenCode's HTTP server with SSE events:

```
Health: GET /global/health
Events: GET /event (SSE, text/event-stream)
Sessions: GET /session
Messages: GET /session/{id}/message
Auth: Basic auth header
```

SSE event types: `session.created`, `session.status`, `session.idle`, `message.part.updated`, `permission.asked`, etc.

### UltraWork Specifics

- Port: `localhost:4096`
- Auth: `opencode:test123` (Basic auth)
- Config: `~/.config/ultrawork/opencode.json`
- Also supports plugin-based integration via `@opencode-ai/plugin`
