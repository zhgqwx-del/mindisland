# Roadmap

## Phase 1: Claude Code MVP ✅

- [x] Tauri 2 project (React 19 + Vite 7 + Tailwind 4)
- [x] System tray + floating panel + auto-hide on blur
- [x] macOS Dock hidden (Accessory policy)
- [x] Claude Code hook adapter (15 event types)
- [x] Hook auto-installer (settings.json management)
- [x] Session discovery from JSONL transcripts
- [x] Permission approval GUI (Deny / Allow Once / Always Allow)
- [x] Bidirectional IPC for PermissionRequest
- [x] Session persistence across restarts
- [x] Notification sound (Glass) + deduplication
- [x] Panel auto-popup on permission request
- [x] Dynamic tray icon (gray/green/red)
- [x] Panel height auto-adapts to content
- [x] Right-click tray menu (Quit)
- [x] Mute toggle in header
- [x] Process liveness monitor
- [x] UI aligned with Open Vibe Island design language

## Phase 2: OpenCode Integration ✅

- [x] OpenCode JS plugin (`hooks/mindisland-opencode-plugin.js`)
- [x] Socket adapter multi-source detection (`_source` / session_id prefix)
- [x] OpenCode plugin auto-installer (copy to `~/.config/opencode/plugins/`)
- [x] Permission reply: plugin POSTs to `/permission/{id}/reply`
- [x] Question reply: plugin POSTs to `/question/{id}/reply`
- [x] Rewrite `opencode.rs` as plugin installer (not SSE adapter)
- [x] Test with live OpenCode session (permission dock verified)
- [x] Assistant last response display in session row
- [x] Mutex discipline: release before side effects (emit/save/tray)
- [x] Phase protection: WaitingForApproval not overwritten by Running
- [x] Subagent detection: use agent_type instead of agent_id
- [ ] Question dock: proper WaitingForAnswer UI (currently uses PermissionRequest path)

## Phase 3: UltraWork + Multi-Agent

- [ ] UltraWork JS plugin (same pattern as OpenCode)
- [ ] Multi-agent display (grouped by agent type)
- [ ] Agent filtering/search
- [ ] Token usage tracking dashboard

## Phase 4: Windows + Release

- [ ] Windows Named Pipe IPC (replace Unix Socket)
- [ ] Windows system tray + panel positioning
- [ ] CI/CD dual-platform build (DMG + NSIS)
- [ ] Auto-updater (tauri-plugin-updater)
- [ ] Proper app icon design

## Phase 5: Advanced

- [ ] Terminal jump-back (AppleScript / Accessibility API)
- [ ] Plugin system for community agent adapters
- [ ] Codex / Gemini CLI / Cursor adapters
- [ ] Multi-window support for multiple monitors
