#!/usr/bin/env bash
# Builds and installs Panorama for the current user on Omarchy / Arch Linux.
set -euo pipefail

SOURCE_DIRECTORY="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BINARY="$HOME/.local/bin/panorama"
SETTINGS_FILE="${XDG_CONFIG_HOME:-$HOME/.config}/panorama/settings.json"

install_packages() {
  echo "==> Installing rust, adb and lm_sensors (sudo)"
  sudo pacman -S --needed rust android-tools lm_sensors
}

# Linux's kernel sensor drivers play LibreHardwareMonitor's role on Windows. CPU (k10temp,
# coretemp) and NVMe temperatures work on their own; the motherboard's Super I/O chip (fans,
# board temperature) needs lm_sensors to find and load its driver.
detect_hardware_sensors() {
  echo "==> Detecting motherboard sensors (sudo)"
  sudo sensors-detect --auto > /dev/null
  if [[ -f /etc/conf.d/lm_sensors ]]; then
    sudo systemctl enable --now lm_sensors.service
  fi
  enable_msi_superio_driver
}

# MSI boards use a Nuvoton NCT6687D, whose kernel driver (nct6683) only loads with force=1.
enable_msi_superio_driver() {
  if ! grep -qi "micro-star" /sys/class/dmi/id/board_vendor 2> /dev/null || grep -qs "^nct6" /sys/class/hwmon/hwmon*/name; then
    return 0
  fi
  echo "==> Enabling the MSI motherboard sensor driver (nct6683 force=1)"
  echo "options nct6683 force=1" | sudo tee /etc/modprobe.d/panorama-nct6683.conf > /dev/null
  echo "nct6683" | sudo tee /etc/modules-load.d/panorama-nct6683.conf > /dev/null
  sudo modprobe nct6683 force=1 || echo "    (could not load nct6683 now; it will load after a reboot)"
}

build_and_install_binary() {
  echo "==> Building (release)"
  cargo build --release --locked --manifest-path "$SOURCE_DIRECTORY/Cargo.toml"
  install -D -m 755 "$SOURCE_DIRECTORY/target/release/panorama" "$BINARY"
}

install_udev_rule() {
  echo "==> Installing udev rule for the screen (sudo)"
  sudo install -m 644 "$SOURCE_DIRECTORY/packaging/70-tryx-panorama.rules" /etc/udev/rules.d/
  sudo udevadm control --reload
  sudo udevadm trigger --subsystem-match=tty --subsystem-match=usb
}

install_initial_settings() {
  if [[ -f "$SETTINGS_FILE" ]]; then
    echo "==> Keeping existing settings at $SETTINGS_FILE"
    return
  fi
  echo "==> Copying your KANALI settings to $SETTINGS_FILE"
  install -D -m 644 "$SOURCE_DIRECTORY/packaging/settings-from-kanali.json" "$SETTINGS_FILE"
}

install_desktop_entry() {
  echo "==> Adding Panorama to the app launcher"
  install -D -m 644 "$SOURCE_DIRECTORY/packaging/panorama.desktop" "$HOME/.local/share/applications/panorama.desktop"
}

install_service() {
  echo "==> Enabling the panorama user service"
  install -D -m 644 "$SOURCE_DIRECTORY/packaging/panorama.service" "$HOME/.config/systemd/user/panorama.service"
  systemctl --user daemon-reload
  systemctl --user enable --now panorama.service
  systemctl --user restart panorama.service
}

install_packages
detect_hardware_sensors
build_and_install_binary
install_udev_rule
install_initial_settings
install_desktop_entry
install_service
echo
echo "Done. Open 'Panorama' from the launcher, or run: panorama"
echo "Service logs: journalctl --user -u panorama -f"
