# MindIsland — Windows release build
$ErrorActionPreference = "Stop"

Write-Host "=== Building MindIsland (release) ===" -ForegroundColor Cyan
bunx tauri build

Write-Host ""
Write-Host "=== Build complete ===" -ForegroundColor Green
Write-Host "Installer: src-tauri\target\release\bundle\nsis\MindIsland_*.exe"
