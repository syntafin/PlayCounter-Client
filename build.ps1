param(
    [switch]$Bundle
)

$env:PATH = "$HOME\.cargo\bin;$HOME\.nodejs;$env:PATH"
$env:INCLUDE = [Environment]::GetEnvironmentVariable("INCLUDE", "User")
$env:LIB = [Environment]::GetEnvironmentVariable("LIB", "User")
$env:LIBPATH = [Environment]::GetEnvironmentVariable("LIBPATH", "User")

Stop-Process -Name "playcounter-client" -Force -ErrorAction SilentlyContinue
Stop-Process -Name "gametracker-client" -Force -ErrorAction SilentlyContinue

if ($Bundle) {
    Write-Host "Erstelle NSIS Windows Setup Installer..." -ForegroundColor Cyan
    npx @tauri-apps/cli build
    Write-Host "Fertig! Installer liegt unter: .\target\release\bundle\nsis\PlayCounter_0.1.0_x64-setup.exe" -ForegroundColor Green
} else {
    Write-Host "Kompiliere PlayCounter Release Binary..." -ForegroundColor Cyan
    npx @tauri-apps/cli build --no-bundle
    Write-Host "Fertig! Die Binary liegt unter: .\target\release\playcounter-client.exe" -ForegroundColor Green
}
