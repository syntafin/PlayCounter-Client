# Development-Modus mit Hot-Reloading starten
$env:PATH = "$HOME\.cargo\bin;$HOME\.nodejs;$env:PATH"
$env:INCLUDE = [Environment]::GetEnvironmentVariable("INCLUDE", "User")
$env:LIB = [Environment]::GetEnvironmentVariable("LIB", "User")
$env:LIBPATH = [Environment]::GetEnvironmentVariable("LIBPATH", "User")

Write-Host "Starte PlayCounter im Entwicklungsmodus (Hot-Reloading)..." -ForegroundColor Cyan
npx @tauri-apps/cli dev
