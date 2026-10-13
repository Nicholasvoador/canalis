# Removes Panorama for the current Windows user. Your settings in %APPDATA%\panorama are kept.
$ErrorActionPreference = 'Stop'

$InstallDirectory = Join-Path $env:LOCALAPPDATA 'Programs\Panorama'
$RunKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
$Shortcut = Join-Path ([Environment]::GetFolderPath('Programs')) 'Panorama.lnk'

Remove-ItemProperty $RunKey -Name Panorama -ErrorAction SilentlyContinue
Get-Process -Name panorama -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 500
Remove-Item $Shortcut -ErrorAction SilentlyContinue
Remove-Item $InstallDirectory -Recurse -Force -ErrorAction SilentlyContinue
Write-Host "Removed. Settings kept in $env:APPDATA\panorama\"
Write-Host 'To go back to KANALI, open it and turn on "Startup with Windows" in its settings.'
