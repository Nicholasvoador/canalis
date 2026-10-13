# Canalis

**Canalis** (Latin for "channel", the most likely meaning of KANALI) is a native replacement for TRYX **KANALI** on **Linux and Windows**, for the **TRYX Panorama** cooler screen. It's written in Rust: one program with a desktop window, a background service and a command line.

- **Window:** laid out like KANALI: a dashboard; a PANORAMA page with the 14 built-in themes in a carousel, your own media, the live stats drawn over either (colour, alignment, rain or smoke filter, CPU/GPU badges, Date & Time), and a preview that can show live screenshots of the cooler; screen and fan settings. It uses egui (native, no Electron, no web view).
- **Background service:** keeps the screen configured and sends it live CPU/GPU/RAM/disk/network stats every second, even when the window is closed. It starts when you log in, like KANALI's "Startup with Windows", and shows a tray icon: click it to open the window, or choose **Quit** to stop it until the next login.
- **Command line:** change any setting from a terminal or a keybinding.

## Install on Linux (Omarchy / Arch)

```bash
./install.sh
```

The script:
- installs `rust`, `android-tools` (adb) and `lm_sensors` with pacman;
- sets up the motherboard's sensor driver so fan speeds and board temperature can be read (`sensors-detect`; on MSI boards it also enables `nct6683` with `force=1`). CPU and NVMe temperatures work without extra setup;
- builds a release binary to `~/.local/bin/canalis`;
- adds a udev rule so your user can reach the screen without root;
- copies your KANALI settings (rotation 270°, your GIF, CPU/GPU temperature overlay);
- adds **Canalis** to the app launcher, with its icon;
- enables `canalis.service`, a systemd **user** service that starts with your desktop session (`graphical-session.target`, which Omarchy's uwsm session provides). Its tray icon appears in Waybar's tray.

Remove it with `./uninstall.sh` (your settings are kept).

## Install on Windows

From PowerShell in this folder:

```powershell
powershell -ExecutionPolicy Bypass -File .\install-windows.ps1
```

Everything it needs comes from winget, so there is nothing to install first. The script:
- installs Rust (`Rustlang.Rustup`), plus the Microsoft C++ build tools Rust needs if they're missing (a large download);
- builds a release binary to `%LOCALAPPDATA%\Programs\Canalis\canalis.exe`;
- turns off KANALI's start with Windows and asks you to quit KANALI (both apps can't share the screen's port);
- copies your KANALI settings;
- adds **Canalis** to the Start menu;
- installs **adb** (`Google.PlatformTools`), used to copy your own images and videos to the screen;
- installs and sets up **[LibreHardwareMonitor](https://github.com/LibreHardwareMonitor/LibreHardwareMonitor)** for CPU temperature, fan speeds and CPU power. Windows only lets kernel drivers read those sensors. After one administrator prompt, it installs the PawnIO driver LibreHardwareMonitor uses, turns on its web server (Canalis reads `http://127.0.0.1:8085/data.json`), and starts it minimised to the tray at login with administrator rights (the `Canalis - LibreHardwareMonitor` scheduled task);
- starts the service at login (a `Canalis` value under `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`) and starts it now.

Running the installer means accepting the licences of the packages it installs from winget.

Remove it with `.\uninstall-windows.ps1`. Your settings are kept, and so are LibreHardwareMonitor, PawnIO and adb (it prints the command to remove them).

## Use

Open **Canalis** from the launcher or Start menu, or run `canalis`. To open on a specific page: `canalis ui --page fan`. The pages are `home`, `specifications`, `presets`, `customization`, `screen`, `fan` and `setting`.

The **SETTING** page starts, restarts and stops the service, and turns **Start at login** on or off.

The command line edits the same settings, and the service applies changes within a second:

```bash
canalis brightness 70
canalis rotate 270                  # 0, 90, 180, 270
canalis screen off                  # or on
canalis presets                     # list built-in themes
canalis preset 12                   # built-in theme 1-14, or: canalis preset custom
canalis custom ~/Pictures/loop.gif --color '#ffffff'   # your own media (sent over adb)
canalis custom a.mp4 --append       # add to the media list
canalis overlay "CPU Temperature" "GPU Temperature"     # up to 3 live stats
canalis fan fixed 60                # or: canalis fan smart
canalis units fahrenheit
canalis show                        # print settings
```

Files:

| | Linux | Windows |
|---|---|---|
| Settings | `~/.config/canalis/settings.json` | `%APPDATA%\canalis\settings.json` |
| Live status (service → window) | `$XDG_RUNTIME_DIR/canalis/status.json` | `%LOCALAPPDATA%\canalis\status.json` |
| Service log | `journalctl --user -u canalis -f` | `%LOCALAPPDATA%\canalis\service.log` |
| Copies of your media, for the preview | `~/.config/canalis/media/` | `%APPDATA%\canalis\media\` |

## How it works

This was worked out from KANALI 1.2.1 with REA, plus the screen's own serial log, which records every message with its raw bytes.

The screen is a small Android 11 device. It connects over USB as `18d1:2d03`, which gives two connections:

| Connection | Device | Used for |
|---|---|---|
| CDC serial | `/dev/ttyACM*` (`/dev/tryx-panorama`) on Linux, `COM*` on Windows | settings and live stats |
| ADB | `adb` | copying media to `/sdcard/pcMedia/` |

Each serial frame looks like this:

```
0x5A | len_hi len_lo | payload | checksum | 0x5A
```

- `len`: size of the whole unescaped frame, including both `0x5A` bytes.
- `checksum`: sum of the length and payload bytes, modulo 256.
- Escaping: `0x5A` becomes `5B 01`, and `0x5B` becomes `5B 02`.

The payload is a text message like an HTTP request: `POST config 1\r\nSeqNumber=2\r\nDate=…\r\nContentType=json\r\nContentLength=606\r\n\r\n{…}`. The screen answers each message with `1 200` and `AckNumber=SeqNumber+1`.

Built-in themes: the PANORAMA has 14, and choosing one sends `{"id": "Pre-set 12: Cyber Bunker", "settings": {"color", "align", "filter": {"value": "Rain" or "Smoke" or null, "opacity"}, "badges": ["CPU Badge", "GPU Badge"]}, "sysinfoDisplay": [...]}` (captured from KANALI's log). The custom theme uses the same `settings` object. A badge is only sent while a stat for that processor is shown, as in KANALI. "Mirror Mode" is not a separate setting: it sets the rotation to 90° (on) or 270° (off). The carousel pictures come from an installed KANALI's `app.asar` (Canalis doesn't ship TRYX's artwork), or from live view screenshots (`adb exec-out screencap -p`).

Custom media is copied to `/sdcard/pcMedia/` over adb, and the screen stacks the live stats from the bottom up (the first one chosen is drawn lowest). KANALI previews media from its own copy (`%APPDATA%\kanali\panorama\media`, with a PNG thumbnail for videos), and crops each file to 2:1, or to 1:1 for its split-screen mode, before sending it. Canalis keeps its own copies the same way, and pulls a file back from the screen the first time it needs one for the preview.

The startup sequence copies KANALI: `POST conn`, then `POST config` (all settings as one JSON body), then `POST waterBlockScreen`. After that the service sends `STATE all` with a hardware snapshot every second, and re-sends `config` whenever the settings file changes. The tests in `src/protocol.rs` check the encoder against real frames captured from the screen.

KANALI reads its stats through the HWiNFO SDK: `lib\BYMonitor64.dll` is HWiNFO's own client library (8.22, signed by REALiX), loaded by a small native addon. The stats keep HWiNFO's sensor names and units, e.g. `Current DL rate` in KB/s and drive `Read Rate` in MB/s. That SDK is licensed to TRYX and Windows-only, so Canalis reads the same values itself, in the same units:
- **Linux:** `/proc`, `/sys/class/hwmon`, `nvidia-smi` and amdgpu.
- **Windows:** the OS for load, clocks, memory, disk and network (via `sysinfo`), `nvidia-smi` for the GPU, and LibreHardwareMonitor for temperatures, fans and CPU power.

| Source file | What it does |
|---|---|
| `protocol.rs` | framing, escaping, checksum, message format |
| `link.rs` | request/response over the serial port; finds the device |
| `device_config.rs` | settings → the `POST config` body |
| `sensors/` | hardware readings for Linux and Windows |
| `service.rs` | the background loop |
| `service_control/` | start/stop and start at login (systemd, or the Windows Run key) |
| `tray/` | the service's tray icon (Windows notification area, or StatusNotifierItem on Linux) |
| `ui/` | the window |

## Tested

On the real screen (firmware V1.0.11):
- **Windows:** installed with `install-windows.ps1` in place of KANALI; the installer sets up adb and LibreHardwareMonitor (with PawnIO) on its own; the service starts hidden with its tray icon, connects on COM3, applies settings and streams live CPU, memory, disk, network and GPU stats plus CPU, motherboard and NVMe temperatures, CPU power and fan speeds; start at login is registered; command-line output works.
- Brightness changes made from the command line reach the screen within a second, and invalid values are refused and not saved.
- The preview pulls the GIF already on the screen over adb and plays it, with the stats in the same order and layout as the screen. Live view shows real screenshots of the screen, even with another (offline) adb device attached. The carousel imports all 14 theme pictures from KANALI 1.2.1. The built-in-theme message matches the one KANALI sent for Pre-set 12 with Rain, a CPU badge and Date & Time, which the screen accepted.

`cargo test` runs the unit tests on Windows, and the code type-checks for `x86_64-unknown-linux-gnu`.

Not tested yet: anything on Omarchy itself: `install.sh`, the Linux sensor readers and the Waybar tray icon.

## Known gaps

- **"turboPump" warning:** the firmware logs `config JSONException--No value for turboPump`. KANALI's own `false` value causes a similar error. Settings still apply.
- **Stats that need special access:** CPU voltage and memory speed are sent as 0. On Linux, CPU power is sent as 0 unless RAPL is readable; on Windows, temperatures, fans and CPU power need LibreHardwareMonitor.
- **Windows command line:** `canalis.exe` is a windowed program, so `cmd` doesn't wait for it and its exit code isn't reported. Output still appears.
- **Not implemented:** firmware updates, the KANALI cloud connection, "turbo pump" mode, and these KANALI PANORAMA features: split screen (two 1:1 media side by side), cropping media to 2:1 or 1:1 before upload, screen recording to make new media, the Low/Mid/High/Full fan speed profiles (Canalis offers their curves as starting points instead), and playing theme videos on hover.

## Development

```bash
cargo test
cargo run              # window
cargo run -- service   # background service
```
