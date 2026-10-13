# Builds and installs Canalis for the current Windows user, in place of KANALI.
# Run from PowerShell:  powershell -ExecutionPolicy Bypass -File .\install-windows.ps1
$ErrorActionPreference = 'Stop'

$SourceDirectory = $PSScriptRoot
$InstallDirectory = Join-Path $env:LOCALAPPDATA 'Programs\Canalis'
$Binary = Join-Path $InstallDirectory 'canalis.exe'
$SettingsFile = Join-Path $env:APPDATA 'canalis\settings.json'
$RunKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
$Shortcut = Join-Path ([Environment]::GetFolderPath('Programs')) 'Canalis.lnk'
# Canalis was called Panorama before; an install under that name is replaced.
$OldInstallDirectory = Join-Path $env:LOCALAPPDATA 'Programs\Panorama'
$OldSettingsDirectory = Join-Path $env:APPDATA 'panorama'
$OldShortcut = Join-Path ([Environment]::GetFolderPath('Programs')) 'Panorama.lnk'

# Rust on Windows links with Microsoft's C++ tools.
function Install-BuildTools {
    $vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
    if ((Test-Path $vswhere) -and (& $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath)) {
        Write-Host '==> C++ build tools are already installed'
        return
    }
    Write-Host '==> Installing the Microsoft C++ build tools Rust needs (a large download; approve the administrator prompt)'
    winget install --id Microsoft.VisualStudio.2022.BuildTools -e --silent --disable-interactivity --accept-package-agreements --accept-source-agreements --override '--quiet --wait --norestart --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended' | Out-Host
    if ($LASTEXITCODE -ne 0) { throw 'winget could not install the C++ build tools' }
}

function Install-Rust {
    $env:PATH = (Join-Path $env:USERPROFILE '.cargo\bin') + ";$env:PATH"
    if (Get-Command cargo -ErrorAction SilentlyContinue) {
        Write-Host '==> Rust is already installed'
        return
    }
    Install-WingetPackage 'Rustlang.Rustup'
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) { rustup default stable | Out-Host }
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) { throw 'Rust was installed but cargo was not found; open a new terminal and run this again' }
}

function Build-Binary {
    Write-Host '==> Building (release)'
    $targetDirectory = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $SourceDirectory 'target' }
    cargo build --release --locked --manifest-path (Join-Path $SourceDirectory 'Cargo.toml') | Out-Host
    if ($LASTEXITCODE -ne 0) { throw 'cargo build failed' }
    return Join-Path $targetDirectory 'release\canalis.exe'
}

function Stop-RunningCanalis {
    Get-Process | Where-Object { $_.Path -like "$InstallDirectory\*" -or $_.Path -like "$OldInstallDirectory\*" } | Stop-Process -Force
    Start-Sleep -Milliseconds 500
}

function Remove-OldPanoramaInstall {
    if (Test-Path $OldInstallDirectory) {
        Write-Host '==> Removing the old Panorama install (now Canalis)'
        Remove-Item $OldInstallDirectory -Recurse -Force
    }
    Remove-ItemProperty $RunKey -Name Panorama -ErrorAction SilentlyContinue
    Remove-Item $OldShortcut -ErrorAction SilentlyContinue
    if ((Test-Path (Join-Path $OldSettingsDirectory 'settings.json')) -and -not (Test-Path (Split-Path $SettingsFile))) {
        Write-Host "==> Moving your settings to $(Split-Path $SettingsFile)"
        Move-Item $OldSettingsDirectory (Split-Path $SettingsFile)
    }
}

function Install-Binary([string]$BuiltBinary) {
    Write-Host "==> Installing to $InstallDirectory"
    New-Item -ItemType Directory -Force $InstallDirectory | Out-Null
    Copy-Item $BuiltBinary $Binary -Force
}

# KANALI and Canalis cannot share the screen's serial port, so KANALI stops starting with Windows.
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
    Write-Host '==> Adding Canalis to the Start menu'
    $link = (New-Object -ComObject WScript.Shell).CreateShortcut($Shortcut)
    $link.TargetPath = $Binary
    $link.WorkingDirectory = $InstallDirectory
    $link.Description = 'Configure the TRYX Panorama cooler screen'
    $link.Save()
}

function Enable-LoginStart {
    Write-Host '==> Starting the service when you log in'
    Set-ItemProperty $RunKey -Name Canalis -Value "`"$Binary`" service"
}

function Start-CanalisService {
    Write-Host '==> Starting the service'
    Start-Process $Binary -ArgumentList 'service' -WindowStyle Hidden
}

function Assert-WingetInstalled {
    if (-not (Get-Command winget -ErrorAction SilentlyContinue)) {
        throw 'winget is missing. Install "App Installer" from the Microsoft Store, then run this again.'
    }
}

function Install-WingetPackage([string]$Id) {
    winget list --id $Id -e --disable-interactivity | Out-Null
    if ($LASTEXITCODE -eq 0) {
        Write-Host "==> $Id is already installed"
        return
    }
    Write-Host "==> Installing $Id"
    winget install --id $Id -e --silent --disable-interactivity --accept-package-agreements --accept-source-agreements | Out-Host
    if ($LASTEXITCODE -ne 0) { throw "winget could not install $Id" }
}

function Find-LibreHardwareMonitor {
    $packages = Join-Path $env:LOCALAPPDATA 'Microsoft\WinGet\Packages'
    Get-ChildItem $packages -Directory -Filter 'LibreHardwareMonitor.LibreHardwareMonitor*' -ErrorAction SilentlyContinue |
        Get-ChildItem -Recurse -Filter 'LibreHardwareMonitor.exe' | Select-Object -First 1 -ExpandProperty FullName
}

# The driver, settings and elevated login task need administrator rights: one UAC prompt.
function Initialize-LibreHardwareMonitor {
    $executable = Find-LibreHardwareMonitor
    if (-not $executable) {
        Write-Warning 'LibreHardwareMonitor.exe not found; temperatures and fan speeds will read 0.'
        return
    }
    Write-Host '==> Setting up LibreHardwareMonitor (approve the administrator prompt)'
    $helper = Join-Path $SourceDirectory 'packaging\windows-hardware-monitor.ps1'
    $arguments = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', "`"$helper`"", '-Executable', "`"$executable`"", '-UserName', "`"$env:USERDOMAIN\$env:USERNAME`"")
    try {
        $helperProcess = Start-Process powershell -Verb RunAs -Wait -PassThru -ArgumentList $arguments
        if ($helperProcess.ExitCode -ne 0) { Write-Warning 'LibreHardwareMonitor setup failed; temperatures and fan speeds will read 0.' }
    } catch {
        Write-Warning 'Administrator prompt declined; temperatures and fan speeds will read 0 until you run this installer again.'
    }
}

# adb copies your own images and videos to the screen; LibreHardwareMonitor provides temperatures and fans.
function Install-Companions {
    Install-WingetPackage 'Google.PlatformTools'
    Install-WingetPackage 'LibreHardwareMonitor.LibreHardwareMonitor'
    Initialize-LibreHardwareMonitor
}

Assert-WingetInstalled
Install-BuildTools
Install-Rust
$builtBinary = Build-Binary
Stop-RunningCanalis
Remove-OldPanoramaInstall
Install-Binary $builtBinary
Disable-Kanali
Install-InitialSettings
Install-StartMenuShortcut
Enable-LoginStart
Install-Companions
Start-CanalisService
Write-Host ''
Write-Host 'Done. Open Canalis from the Start menu.'
