# Builds and installs Panorama for the current Windows user, in place of KANALI.
# Run from PowerShell:  powershell -ExecutionPolicy Bypass -File .\install-windows.ps1
$ErrorActionPreference = 'Stop'

$SourceDirectory = $PSScriptRoot
$InstallDirectory = Join-Path $env:LOCALAPPDATA 'Programs\Panorama'
$Binary = Join-Path $InstallDirectory 'panorama.exe'
$SettingsFile = Join-Path $env:APPDATA 'panorama\settings.json'
$RunKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
$Shortcut = Join-Path ([Environment]::GetFolderPath('Programs')) 'Panorama.lnk'

function Assert-RustInstalled {
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        throw 'Rust is not installed. Install it with: winget install Rustlang.Rustup   (then open a new terminal)'
    }
}

function Build-Binary {
    Write-Host '==> Building (release)'
    $targetDirectory = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $SourceDirectory 'target' }
    cargo build --release --locked --manifest-path (Join-Path $SourceDirectory 'Cargo.toml') | Out-Host
    if ($LASTEXITCODE -ne 0) { throw 'cargo build failed' }
    return Join-Path $targetDirectory 'release\panorama.exe'
}

function Stop-RunningPanorama {
    Get-Process -Name panorama -ErrorAction SilentlyContinue | Stop-Process -Force
    Start-Sleep -Milliseconds 500
}

function Install-Binary([string]$BuiltBinary) {
    Write-Host "==> Installing to $InstallDirectory"
    New-Item -ItemType Directory -Force $InstallDirectory | Out-Null
    Copy-Item $BuiltBinary $Binary -Force
}

# KANALI and Panorama cannot share the screen's serial port, so KANALI stops starting with Windows.
function Disable-Kanali {
    if (Get-ItemProperty $RunKey -Name KANALI -ErrorAction SilentlyContinue) {
        Write-Host '==> Turning off KANALI''s start with Windows (turn it back on in KANALI''s settings to undo)'
        Remove-ItemProperty $RunKey -Name KANALI
    }
    $kanali = Get-Process -Name KANALI -ErrorAction SilentlyContinue
    if (-not $kanali) { return }
    Write-Host '==> Closing KANALI'
    $kanali | Stop-Process -Force -ErrorAction SilentlyContinue
    Start-Sleep -Seconds 2
    # KANALI runs as administrator, so a normal user may not be allowed to close it.
    while (Get-Process -Name KANALI -ErrorAction SilentlyContinue) {
        Read-Host 'KANALI is still running. Quit it from its tray icon (right-click > Exit), then press Enter'
    }
}

function Install-InitialSettings {
    if (Test-Path $SettingsFile) {
        Write-Host "==> Keeping existing settings at $SettingsFile"
        return
    }
    Write-Host "==> Copying your KANALI settings to $SettingsFile"
    New-Item -ItemType Directory -Force (Split-Path $SettingsFile) | Out-Null
    Copy-Item (Join-Path $SourceDirectory 'packaging\settings-from-kanali.json') $SettingsFile
}

function Install-StartMenuShortcut {
    Write-Host '==> Adding Panorama to the Start menu'
    $link = (New-Object -ComObject WScript.Shell).CreateShortcut($Shortcut)
    $link.TargetPath = $Binary
    $link.WorkingDirectory = $InstallDirectory
    $link.Description = 'Configure the TRYX Panorama cooler screen'
    $link.Save()
}

function Enable-LoginStart {
    Write-Host '==> Starting the service when you log in'
    Set-ItemProperty $RunKey -Name Panorama -Value "`"$Binary`" service"
}

function Start-PanoramaService {
    Write-Host '==> Starting the service'
    Start-Process $Binary -ArgumentList 'service' -WindowStyle Hidden
}

function Show-OptionalExtras {
    if (-not (Get-Command adb -ErrorAction SilentlyContinue) -and -not (Test-Path "$env:LOCALAPPDATA\Microsoft\WinGet\Links\adb.exe")) {
        Write-Host ''
        Write-Host 'Optional: to add your own images and videos, install adb:  winget install Google.PlatformTools'
    }
    Write-Host ''
    Write-Host 'Optional: for CPU temperature, fan speeds and CPU power, install LibreHardwareMonitor:'
    Write-Host '  winget install LibreHardwareMonitor.LibreHardwareMonitor'
    Write-Host '  then run it as administrator and turn on Options > Remote Web Server > Run, and Options > Run On Windows Startup.'
}

Assert-RustInstalled
$builtBinary = Build-Binary
Stop-RunningPanorama
Install-Binary $builtBinary
Disable-Kanali
Install-InitialSettings
Install-StartMenuShortcut
Enable-LoginStart
Start-PanoramaService
Show-OptionalExtras
Write-Host ''
Write-Host 'Done. Open Panorama from the Start menu.'
