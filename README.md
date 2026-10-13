<p align="center"><img src="docs/banner.svg" alt="Canalis: control the TRYX Panorama cooler screen on Windows and Linux" width="100%"></p>

A lightweight replacement for TRYX **KANALI** that controls the **TRYX Panorama** cooler screen on Windows and Linux.

## Screenshots

| Dashboard | Built-in themes |
| --- | --- |
| ![The dashboard: network graphs, CPU and GPU load gauges, memory and disk bars](docs/screenshots/home.png) | ![The PANORAMA page: a carousel of the 14 built-in themes and the stats options](docs/screenshots/presets.png) |
| **Your own media** | **Fan** |
| ![Customization: a preview of the screen with live stats over your own GIF, and your media list](docs/screenshots/customization.png) | ![Fan settings: smart mode with an editable fan curve, or a fixed speed](docs/screenshots/fan.png) |

## Install

Download the latest build from [Releases](https://github.com/sallen2/canalis/releases/latest).

**Windows:** unzip `canalis-…-windows-x64.zip` and run this from PowerShell in that folder:

```powershell
powershell -ExecutionPolicy Bypass -File .\install-windows.ps1
```

**Linux (Arch / Omarchy):** extract `canalis-…-linux-x64.tar.gz` and run:

```bash
./install.sh
```

The installer sets up everything Canalis needs, turns off KANALI's start at login, and starts Canalis when you log in. Run `uninstall-windows.ps1` or `uninstall.sh` to remove it; your settings are kept.

## Use

Open **Canalis** from the Start menu or app launcher. A background service keeps the screen updated with live stats, even when the window is closed; its tray icon reopens the window.

Settings can also be changed from a terminal:

```bash
canalis preset 12        # built-in theme 1-14
canalis brightness 70
canalis fan fixed 60     # or: canalis fan smart
canalis show             # print all settings
```

Run `canalis --help` for the rest.

## Build from source

```bash
cargo test
cargo run --release
```

The install scripts build Canalis themselves when run from a copy of this repository. [How it works](docs/how-it-works.md) covers the screen's protocol, what was learned from KANALI, and known gaps.
