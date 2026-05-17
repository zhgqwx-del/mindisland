#!/bin/bash
# MindIsland — macOS setup script
set -e

echo "=== MindIsland Setup ==="

# Check prerequisites
command -v bun >/dev/null 2>&1 || { echo "Error: bun is required. Install from https://bun.sh"; exit 1; }
command -v cargo >/dev/null 2>&1 || { echo "Error: cargo is required. Install from https://rustup.rs"; exit 1; }

# Install frontend dependencies
echo "Installing frontend dependencies..."
bun install

# Check Rust compilation
echo "Checking Rust build..."
cd src-tauri && cargo check && cd ..

# Install Claude Code hooks (if Claude Code is installed)
if [ -d "$HOME/.claude" ]; then
    echo "Claude Code detected. Hooks will be auto-installed on first launch."
else
    echo "Claude Code not found. Install it to enable monitoring."
fi

echo ""
echo "=== Setup complete ==="
echo ""
echo "Commands:"
echo "  ./dev.sh        — Start development mode"
echo "  ./build.sh      — Build release app"
echo "  killall mindisland  — Stop MindIsland"
echo ""
