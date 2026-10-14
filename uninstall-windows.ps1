# Removes Canalis for the current Windows user. Your settings in %APPDATA%\canalis are kept,
# and so are LibreHardwareMonitor, PawnIO, adb and ffmpeg (other apps may use them).
param(
    # Set by the setup exe's uninstaller, which runs from the program's folder and removes it itself.
    [switch]$FromUninstaller
)
$ErrorActionPreference = 'Stop'

$InstallDirectory = Join-Path $env:LOCALAPPDATA 'Programs\Canalis'
$RunKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
$Shortcut = Join-Path ([Environment]::GetFolderPath('Programs')) 'Canalis.lnk'
$HardwareMonitorTask = 'Canalis - LibreHardwareMonitor'

Remove-ItemProperty $RunKey -Name Canalis -ErrorAction SilentlyContinue
Get-Process canalis -ErrorAction SilentlyContinue | Where-Object { $_.Path -like "$InstallDirectory\*" } | Stop-Process -Force
Start-Sleep -Milliseconds 500
Remove-Item $Shortcut -ErrorAction SilentlyContinue
if (-not $FromUninstaller) { Remove-Item $InstallDirectory -Recurse -Force -ErrorAction SilentlyContinue }
if (Get-ScheduledTask -TaskName $HardwareMonitorTask -ErrorAction SilentlyContinue) {
    Write-Host 'Removing the LibreHardwareMonitor login task (approve the administrator prompt)'
    try {
        Start-Process powershell -Verb RunAs -Wait -ArgumentList @('-NoProfile', '-Command', "Unregister-ScheduledTask -TaskName '$HardwareMonitorTask' -Confirm:`$false")
    } catch {
        Write-Warning "Administrator prompt declined; remove the '$HardwareMonitorTask' task in Task Scheduler."
    }
}
Write-Host "Removed. Settings kept in $env:APPDATA\canalis\"
Write-Host 'To remove the helpers too: winget uninstall LibreHardwareMonitor.LibreHardwareMonitor namazso.PawnIO Google.PlatformTools Gyan.FFmpeg'
Write-Host 'To go back to KANALI, open it and turn on "Startup with Windows" in its settings.'
