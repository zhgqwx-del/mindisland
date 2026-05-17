# MindIsland — Windows setup script
$ErrorActionPreference = "Stop"

Write-Host "=== MindIsland Setup ===" -ForegroundColor Cyan

# Check prerequisites
if (!(Get-Command bun -ErrorAction SilentlyContinue)) {
    Write-Host "Error: bun is required. Install from https://bun.sh" -ForegroundColor Red
    exit 1
}
if (!(Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "Error: cargo is required. Install from https://rustup.rs" -ForegroundColor Red
    exit 1
}

# Install frontend dependencies
Write-Host "Installing frontend dependencies..."
bun install

# Check Rust compilation
Write-Host "Checking Rust build..."
Push-Location src-tauri
cargo check
Pop-Location

Write-Host ""
Write-Host "=== Setup complete ===" -ForegroundColor Green
Write-Host ""
Write-Host "Commands:"
Write-Host "  .\dev.ps1        — Start development mode"
Write-Host "  .\build.ps1      — Build release app"
