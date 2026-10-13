#!/usr/bin/env bash
# Removes Panorama. Your settings in ~/.config/panorama are kept, and so are lm_sensors and adb.
set -euo pipefail

systemctl --user disable --now panorama.service 2>/dev/null || true
rm -f "$HOME/.config/systemd/user/panorama.service"
systemctl --user daemon-reload
rm -f "$HOME/.local/bin/panorama" "$HOME/.local/share/applications/panorama.desktop"
sudo rm -f /etc/udev/rules.d/70-tryx-panorama.rules
sudo udevadm control --reload
sudo rm -f /etc/modprobe.d/panorama-nct6683.conf /etc/modules-load.d/panorama-nct6683.conf
echo "Removed. Settings kept in ${XDG_CONFIG_HOME:-$HOME/.config}/panorama/"
