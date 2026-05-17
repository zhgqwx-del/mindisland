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

## Phase 2: OpenCode + Polish

- [ ] OpenCode SSE adapter (re-enable `agents/opencode.rs`)
- [ ] Claude Code fork support (Qoder/Qwen/Factory — same hook format)
- [ ] AskUserQuestion tool support (structured question UI)
- [ ] Subagent tracking (show active subagent list)
- [ ] Settings panel (sound selection, notification preferences)

## Phase 3: UltraWork + Multi-Agent

- [ ] Re-enable UltraWork SSE adapter
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
