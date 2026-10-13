#!/usr/bin/env bash
# Builds and installs Panorama for the current user on Omarchy / Arch Linux.
set -euo pipefail

SOURCE_DIRECTORY="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BINARY="$HOME/.local/bin/panorama"
SETTINGS_FILE="${XDG_CONFIG_HOME:-$HOME/.config}/panorama/settings.json"

install_packages() {
  echo "==> Installing rust and adb (sudo)"
  sudo pacman -S --needed rust android-tools
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
build_and_install_binary
install_udev_rule
install_initial_settings
install_desktop_entry
install_service
echo
echo "Done. Open 'Panorama' from the launcher, or run: panorama"
echo "Service logs: journalctl --user -u panorama -f"
