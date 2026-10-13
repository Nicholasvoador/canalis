# Panorama

A native replacement for TRYX **KANALI** on **Linux and Windows**, for the **TRYX Panorama** cooler screen. It's written in Rust: one program with a desktop window, a background service and a command line.

- **Window:** a dashboard, plus screen, theme and fan-curve settings, laid out like KANALI. It uses egui (native, no Electron, no web view).
- **Background service:** keeps the screen configured and sends it live CPU/GPU/RAM/disk/network stats every second, even when the window is closed. It starts when you log in, like KANALI's "Startup with Windows", and shows a tray icon: click it to open the window, or choose **Quit** to stop it until the next login.
- **Command line:** change any setting from a terminal or a keybinding.

## Install on Linux (Omarchy / Arch)

```bash
./install.sh
```

The script:
- installs `rust`, `android-tools` (adb) and `lm_sensors` with pacman;
- sets up the motherboard's sensor driver so fan speeds and board temperature can be read (`sensors-detect`; on MSI boards it also enables `nct6683` with `force=1`). CPU and NVMe temperatures work without extra setup;
- builds a release binary to `~/.local/bin/panorama`;
- adds a udev rule so your user can reach the screen without root;
- copies your KANALI settings (rotation 270°, your GIF, CPU/GPU temperature overlay);
- adds **Panorama** to the app launcher;
- enables `panorama.service`, a systemd **user** service that starts with your desktop session (`graphical-session.target`, which Omarchy's uwsm session provides). Its tray icon appears in Waybar's tray.

Remove it with `./uninstall.sh` (your settings are kept).

## Install on Windows

From PowerShell in this folder:

```powershell
powershell -ExecutionPolicy Bypass -File .\install-windows.ps1
```

Everything it needs comes from winget, so there is nothing to install first. The script:
- installs Rust (`Rustlang.Rustup`), plus the Microsoft C++ build tools Rust needs if they're missing (a large download);
- builds a release binary to `%LOCALAPPDATA%\Programs\Panorama\panorama.exe`;
- turns off KANALI's start with Windows and asks you to quit KANALI (both apps can't share the screen's port);
- copies your KANALI settings;
- adds **Panorama** to the Start menu;
- installs **adb** (`Google.PlatformTools`), used to copy your own images and videos to the screen;
- installs and sets up **[LibreHardwareMonitor](https://github.com/LibreHardwareMonitor/LibreHardwareMonitor)** for CPU temperature, fan speeds and CPU power. Windows only lets kernel drivers read those sensors. After one administrator prompt, it installs the PawnIO driver LibreHardwareMonitor uses, turns on its web server (Panorama reads `http://127.0.0.1:8085/data.json`), and starts it minimised to the tray at login with administrator rights (the `Panorama - LibreHardwareMonitor` scheduled task);
- starts the service at login (a `Panorama` value under `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`) and starts it now.

Running the installer means accepting the licences of the packages it installs from winget.

Remove it with `.\uninstall-windows.ps1`. Your settings are kept, and so are LibreHardwareMonitor, PawnIO and adb (it prints the command to remove them).

## Use

Open **Panorama** from the launcher or Start menu, or run `panorama`. To open on a specific page: `panorama ui --page fan`. The pages are `home`, `specifications`, `screen`, `theme`, `fan` and `setting`.

The **SETTING** page starts, restarts and stops the service, and turns **Start at login** on or off.

The command line edits the same settings, and the service applies changes within a second:

```bash
panorama brightness 70
panorama rotate 270                  # 0, 90, 180, 270
panorama screen off                  # or on
panorama presets                     # list built-in themes
panorama preset 2                    # built-in theme 1-7, or: panorama preset custom
panorama custom ~/Pictures/loop.gif --color '#ffffff'   # your own media (sent over adb)
panorama custom a.mp4 --append       # add to the media list
panorama overlay "CPU Temperature" "GPU Temperature"     # up to 3 live stats
panorama fan fixed 60                # or: panorama fan smart
panorama units fahrenheit
panorama show                        # print settings
```

Files:

| | Linux | Windows |
|---|---|---|
| Settings | `~/.config/panorama/settings.json` | `%APPDATA%\panorama\settings.json` |
| Live status (service → window) | `$XDG_RUNTIME_DIR/panorama/status.json` | `%LOCALAPPDATA%\panorama\status.json` |
| Service log | `journalctl --user -u panorama -f` | `%LOCALAPPDATA%\panorama\service.log` |

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

The startup sequence copies KANALI: `POST conn`, then `POST config` (all settings as one JSON body), then `POST waterBlockScreen`. After that the service sends `STATE all` with a hardware snapshot every second, and re-sends `config` whenever the settings file changes. The tests in `src/protocol.rs` check the encoder against real frames captured from the screen.

KANALI reads its stats through the HWiNFO SDK: `lib\BYMonitor64.dll` is HWiNFO's own client library (8.22, signed by REALiX), loaded by a small native addon. The stats keep HWiNFO's sensor names and units, e.g. `Current DL rate` in KB/s and drive `Read Rate` in MB/s. That SDK is licensed to TRYX and Windows-only, so Panorama reads the same values itself, in the same units:
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

`cargo test` runs the unit tests on Windows, and the code type-checks for `x86_64-unknown-linux-gnu`.

Not tested yet: anything on Omarchy itself: `install.sh`, the Linux sensor readers and the Waybar tray icon.

## Known gaps

- **"turboPump" warning:** the firmware logs `config JSONException--No value for turboPump`. KANALI's own `false` value causes a similar error. Settings still apply.
- **Preset theme format:** the exact `id` object for built-in presets is inferred; only the "Customization" format was seen on the wire.
- **Stats that need special access:** CPU voltage and memory speed are sent as 0. On Linux, CPU power is sent as 0 unless RAPL is readable; on Windows, temperatures, fans and CPU power need LibreHardwareMonitor.
- **Windows command line:** `panorama.exe` is a windowed program, so `cmd` doesn't wait for it and its exit code isn't reported. Output still appears.
- **Not implemented:** firmware updates, the KANALI cloud connection, and "turbo pump" mode.

## Development

```bash
cargo test
cargo run              # window
cargo run -- service   # background service
```
