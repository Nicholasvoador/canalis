# Run elevated by install-windows.ps1 (one UAC prompt). Sets up LibreHardwareMonitor, which
# Panorama reads temperatures, fan speeds and CPU power from:
#   - installs PawnIO, the signed kernel driver LibreHardwareMonitor reads sensors through;
#   - turns on its web server (Panorama reads http://127.0.0.1:8085/data.json), minimised to the tray;
#   - starts it at login with administrator rights via a scheduled task, and starts it now.
param(
    [Parameter(Mandatory)] [string]$Executable,
    [Parameter(Mandatory)] [string]$UserName
)
$ErrorActionPreference = 'Stop'

$TaskName = 'Panorama - LibreHardwareMonitor'
$Settings = [ordered]@{
    runWebServerMenuItem = 'true'
    listenerPort         = '8085'
    startMinMenuItem     = 'true'
    minTrayMenuItem      = 'true'
    minCloseMenuItem     = 'true'
}

function Install-PawnIO {
    winget list --id namazso.PawnIO -e --disable-interactivity | Out-Null
    if ($LASTEXITCODE -eq 0) { return }
    Write-Host '==> Installing the PawnIO driver'
    winget install --id namazso.PawnIO -e --silent --disable-interactivity --accept-package-agreements --accept-source-agreements
}

# LibreHardwareMonitor saves its settings when it exits, so it must not be running while they change.
function Stop-LibreHardwareMonitor {
    Get-Process -Name LibreHardwareMonitor -ErrorAction SilentlyContinue | Stop-Process -Force
    Start-Sleep -Seconds 1
}

function Set-LibreHardwareMonitorSettings {
    $configFile = [IO.Path]::ChangeExtension($Executable, '.config')
    $document = New-Object Xml.XmlDocument
    if (Test-Path $configFile) {
        $document.Load($configFile)
    } else {
        $document.LoadXml('<?xml version="1.0" encoding="utf-8"?><configuration><appSettings /></configuration>')
    }
    $appSettings = $document.SelectSingleNode('/configuration/appSettings')
    foreach ($key in $Settings.Keys) {
        $entry = $appSettings.SelectSingleNode("add[@key='$key']")
        if (-not $entry) {
            $entry = $document.CreateElement('add')
            $entry.SetAttribute('key', $key)
            $appSettings.AppendChild($entry) | Out-Null
        }
        $entry.SetAttribute('value', $Settings[$key])
    }
    $document.Save($configFile)
}

function Register-LoginTask {
    $action = New-ScheduledTaskAction -Execute $Executable -WorkingDirectory (Split-Path $Executable)
    $trigger = New-ScheduledTaskTrigger -AtLogOn -User $UserName
    $principal = New-ScheduledTaskPrincipal -UserId $UserName -LogonType Interactive -RunLevel Highest
    $options = New-ScheduledTaskSettingsSet -ExecutionTimeLimit ([TimeSpan]::Zero) -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries
    Register-ScheduledTask -TaskName $TaskName -Action $action -Trigger $trigger -Principal $principal -Settings $options -Force | Out-Null
}

try {
    Install-PawnIO
    Stop-LibreHardwareMonitor
    Set-LibreHardwareMonitorSettings
    Register-LoginTask
    Start-ScheduledTask -TaskName $TaskName
    Write-Host '==> LibreHardwareMonitor is running and will start at login'
} catch {
    Write-Host "LibreHardwareMonitor setup failed: $_" -ForegroundColor Red
    Read-Host 'Press Enter to close'
    exit 1
}
