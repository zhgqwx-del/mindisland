# Lessons Learned

Technical conclusions verified during development, useful for future reference.

## Claude Code Hooks

- **Payload format**: snake_case fields (`session_id`, `hook_event_name`, `tool_name`, `tool_input`)
- **Hook registration**: `~/.claude/settings.json` `hooks` field; each event supports multiple hooks (MindIsland coexists with Vibe Island)
- **Hot reload**: settings.json is read at Claude Code process startup. Mid-session changes require a new Claude Code session
- **Subagent hooks**: Payloads with `agent_id` set are from subagents — skip them (parent session gets SubagentStart/Stop)
- **PermissionRequest timeout**: Must set `"timeout": 86400` in hook config, otherwise Claude Code won't wait for GUI response

## Unix Socket IPC

- **Socket path**: Use `/tmp/mindisland-claude.sock` (not `std::env::temp_dir()` which gives `/var/folders/...`)
- **Startup race**: MindIsland socket bind takes ~500ms after app launch. Hook script needs retry logic (`ConnectionRefusedError` → sleep 0.5s → retry)
- **Hook script**: Must use temp file to pass stdin JSON (bash `echo "$PAYLOAD"` corrupts JSON with special chars). Python3 reads the file and sends via socket
- **Bidirectional**: For PermissionRequest, keep socket connection open. Use `tokio::sync::oneshot` channel to bridge async wait → user click → socket response

## Tauri 2 Panel Window

- **Dock hiding**: `app.set_activation_policy(tauri::ActivationPolicy::Accessory)` is the reliable method. `LSUIElement` in Info.plist gets reset on rebuild
- **Transparent window**: Requires `macOSPrivateApi: true` in tauri.conf.json + `transparent(true)` on window builder + `background_color` set to match content
- **White corners**: Set window `background_color` to `tauri_utils::config::Color(13, 13, 15, 255)` (#0d0d0f) to match CSS background
- **Tray icon**: Needs `image-png` feature in Cargo.toml for `Image::from_bytes()`
- **Panel auto-size**: Use DOM `scrollHeight` measurement via `requestAnimationFrame` + 50ms delay. Container must NOT use `h-screen` (forces full height). Use `resize_panel` Tauri command
- **emit to hidden windows**: Unreliable. Use triple redundancy: `emit` + `panel-opened` event + 2s polling
- **Right-click menu**: Use `menu_on_left_click(false)` to separate left-click (toggle panel) from right-click (context menu)

## UI/UX

- **Workspace name**: Skip common subdirectory names (src-tauri, packages, client, etc.) when extracting from cwd path
- **UTF-8 clip**: Use `str.chars().count()` and `.chars().take(n)` for truncation — byte-level `&s[..n]` panics on multi-byte characters (Chinese, emoji)
- **Notification dedup**: Track `last_attention_session` — only play sound when a NEW session enters waiting state, not on every poll cycle
- **Session visibility**: Only show active + recently completed (<2 min) sessions. Transcript-discovered completed sessions should not clutter the panel
- **Process liveness**: Check `pgrep -f claude` every 15 seconds. Mark sessions as completed if no Claude process found

## UltraWork / OpenCode SSE (Verified)

- **SSE endpoint**: `GET http://localhost:4096/event` with Basic auth `opencode:test123`
- **SSE format**: `data: {"type":"...", "properties":{...}}\n\n`
- **Existing sessions**: `GET /session` returns list; `GET /session/{id}/message` returns messages
- **Plugin system**: `~/.config/ultrawork/opencode.json` supports `"plugin": ["./plugins/xxx.mjs"]` for event hook plugins
- **Plugin spec format**: Must use `"./plugins/xxx.mjs"` (relative path), NOT `"file:./plugins/..."` — OpenCode's `isPathPluginSpec()` only matches `./`, `file://`, or absolute paths
- **OPENCODE_PURE**: UltraWork does NOT set this flag, so plugins are enabled
