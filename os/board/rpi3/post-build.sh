#!/bin/sh
# Raspberry Pi specific steps, run after os/board/post-build.sh. $1 = target rootfs.
set -e
TARGET="$1"

# Wi-Fi for local builds: the network name and password come from the local, untracked .secrets/wifi.conf
# (release images built on GitHub have none: the user writes them in tvos.txt on the boot partition)
# (line 1 = network name, line 2 = password). Only the derived 256-bit key is stored in the image, not the password.
if [ -f "${TVOS_WIFI_FILE:-}" ]; then
    python3 - "$TVOS_WIFI_FILE" "$TARGET/etc/wpa_supplicant.conf" <<'PY'
import hashlib, sys
lines = open(sys.argv[1], encoding="utf-8").read().split("\n")
ssid, password = lines[0].strip(), lines[1].strip()
psk = hashlib.pbkdf2_hmac("sha1", password.encode(), ssid.encode(), 4096, 32).hex()
open(sys.argv[2], "w", encoding="utf-8").write(
    "ctrl_interface=/var/run/wpa_supplicant\n"
    "update_config=0\n"
    "country=IT\n\n"
    "network={\n"
    f'    ssid="{ssid}"\n'
    f"    psk={psk}\n"
    "    key_mgmt=WPA-PSK\n"
    "}\n"
)
PY
    chmod 0600 "$TARGET/etc/wpa_supplicant.conf"
fi

rm -f "$TARGET/etc/init.d/S42wifi"
chmod 0755 "$TARGET/etc/init.d/S02zram" "$TARGET/etc/init.d/S40config" "$TARGET/etc/init.d/S41network" "$TARGET/usr/share/tvos/udhcpc.script"

# Settings file on the boot partition, editable from any PC (Buildroot's post-image puts every rpi-firmware/* file there)
install -m 0644 "$(dirname "$0")/tvos.txt" "$BINARIES_DIR/rpi-firmware/tvos.txt"
