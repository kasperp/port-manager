# Install Port Manager to Windows Startup
# Run as: powershell -ExecutionPolicy Bypass -File Install.ps1

$scriptPath = $PSScriptRoot
$vbsPath = Join-Path $scriptPath "PortManager.vbs"
$startupFolder = [Environment]::GetFolderPath('Startup')
$shortcutPath = Join-Path $startupFolder "PortManager.lnk"

# Verify PortManager.vbs exists
if (-not (Test-Path $vbsPath)) {
    Write-Host "Error: PortManager.vbs not found in $scriptPath" -ForegroundColor Red
    exit 1
}

# Create shortcut in Startup folder
$shell = New-Object -ComObject WScript.Shell
$shortcut = $shell.CreateShortcut($shortcutPath)
$shortcut.TargetPath = $vbsPath
$shortcut.WorkingDirectory = $scriptPath
$shortcut.Description = "Port Manager - SSH Port Forwarding"
$shortcut.Save()

Write-Host "Port Manager installed to Windows Startup" -ForegroundColor Green
Write-Host "Shortcut created: $shortcutPath" -ForegroundColor Cyan
Write-Host ""
Write-Host "The application will start automatically on next login."
Write-Host "To uninstall, run: Uninstall.ps1"
