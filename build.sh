#!/bin/bash
# MindIsland — macOS release build
set -e

echo "=== Building MindIsland (release) ==="

bunx tauri build 2>&1 | tail -5

echo ""
echo "=== Build complete ==="
echo "App: src-tauri/target/release/bundle/macos/MindIsland.app"
echo "DMG: src-tauri/target/release/bundle/dmg/MindIsland_*.dmg"
