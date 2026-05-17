# MindIsland — Windows development mode
$ErrorActionPreference = "Stop"

# Kill existing instance
Get-Process mindisland -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep 1

Write-Host "Building and launching MindIsland (debug)..." -ForegroundColor Cyan
bunx tauri build --debug --bundles nsis

$exe = "src-tauri\target\debug\mindisland.exe"
if (Test-Path $exe) {
    Start-Process $exe
    Write-Host ""
    Write-Host "MindIsland is running." -ForegroundColor Green
} else {
    Write-Host "Build failed." -ForegroundColor Red
    exit 1
}
