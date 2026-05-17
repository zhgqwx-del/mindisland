#!/bin/bash
# MindIsland — macOS development mode
set -e

# Kill existing instance and any hanging hook processes
killall mindisland 2>/dev/null || true
# Kill PermissionRequest hook processes still waiting on the old socket
pkill -f "mindisland-claude-hook" 2>/dev/null || true
rm -f /tmp/mindisland-claude.sock
sleep 1

echo "Building and launching MindIsland (debug)..."
bunx tauri build --debug --bundles app 2>&1 | tail -3

APP="src-tauri/target/debug/bundle/macos/MindIsland.app"
if [ -d "$APP" ]; then
    open "$APP"
    echo ""
    echo "MindIsland is running."
    echo "  Left-click tray icon  → open panel"
    echo "  Right-click tray icon → quit"
else
    echo "Build failed."
    exit 1
fi
