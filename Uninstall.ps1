# Uninstall Port Manager from Windows Startup
# Run as: powershell -ExecutionPolicy Bypass -File Uninstall.ps1

$startupFolder = [Environment]::GetFolderPath('Startup')
$shortcutPath = Join-Path $startupFolder "PortManager.lnk"

if (Test-Path $shortcutPath) {
    Remove-Item $shortcutPath -Force
    Write-Host "Port Manager removed from Windows Startup" -ForegroundColor Green
} else {
    Write-Host "Port Manager was not installed in Startup" -ForegroundColor Yellow
}
