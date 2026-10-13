# Removes Panorama for the current Windows user. Your settings in %APPDATA%\panorama are kept,
# and so are LibreHardwareMonitor, PawnIO and adb (other apps may use them).
$ErrorActionPreference = 'Stop'

$InstallDirectory = Join-Path $env:LOCALAPPDATA 'Programs\Panorama'
$RunKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
$Shortcut = Join-Path ([Environment]::GetFolderPath('Programs')) 'Panorama.lnk'
$HardwareMonitorTask = 'Panorama - LibreHardwareMonitor'

Remove-ItemProperty $RunKey -Name Panorama -ErrorAction SilentlyContinue
Get-Process | Where-Object { $_.Path -like "$InstallDirectory\*" } | Stop-Process -Force
Start-Sleep -Milliseconds 500
Remove-Item $Shortcut -ErrorAction SilentlyContinue
Remove-Item $InstallDirectory -Recurse -Force -ErrorAction SilentlyContinue
if (Get-ScheduledTask -TaskName $HardwareMonitorTask -ErrorAction SilentlyContinue) {
    Write-Host 'Removing the LibreHardwareMonitor login task (approve the administrator prompt)'
    try {
        Start-Process powershell -Verb RunAs -Wait -ArgumentList @('-NoProfile', '-Command', "Unregister-ScheduledTask -TaskName '$HardwareMonitorTask' -Confirm:`$false")
    } catch {
        Write-Warning "Administrator prompt declined; remove the '$HardwareMonitorTask' task in Task Scheduler."
    }
}
Write-Host "Removed. Settings kept in $env:APPDATA\panorama\"
Write-Host 'To remove the helpers too: winget uninstall LibreHardwareMonitor.LibreHardwareMonitor namazso.PawnIO Google.PlatformTools'
Write-Host 'To go back to KANALI, open it and turn on "Startup with Windows" in its settings.'
