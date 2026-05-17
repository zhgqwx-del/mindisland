# Getting Started

## Prerequisites

- [Bun](https://bun.sh) 1.3+
- [Rust](https://rustup.rs) 1.75+ (with cargo)
- Xcode Command Line Tools (macOS)

## Setup

```bash
git clone <repo-url> mindisland
cd mindisland
./setup.sh          # macOS
# .\setup.ps1       # Windows
```

## Development

```bash
./dev.sh            # Build debug + launch app
```

- **Left-click** tray icon → open/close panel
- **Right-click** tray icon → Quit MindIsland

## Build Release

```bash
./build.sh          # macOS: .app + .dmg
# .\build.ps1       # Windows: .exe + NSIS installer
```

Output:
- macOS: `src-tauri/target/release/bundle/macos/MindIsland.app`
- Windows: `src-tauri/target/release/bundle/nsis/MindIsland_*.exe`

## Claude Code Hook

On first launch, MindIsland auto-installs hooks in `~/.claude/settings.json`. The hook script is at `hooks/mindisland-claude-hook.sh`.

To manually manage hooks:
```bash
# Check status (in browser devtools or via Tauri command)
invoke("get_hook_status")

# Reinstall
invoke("install_hooks")

# Remove
invoke("uninstall_hooks")
```

## Troubleshooting

**Panel doesn't show sessions:**
- Check socket: `ls -la /tmp/mindisland-claude.sock`
- Check hooks registered: `grep mindisland ~/.claude/settings.json`
- New Claude Code sessions needed — hooks load on session start

**Multiple tray icons:**
- Kill all instances: `killall mindisland`
- Relaunch: `./dev.sh`

**White corners on panel:**
- Requires `macOSPrivateApi: true` in tauri.conf.json (already configured)
