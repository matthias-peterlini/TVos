#!/bin/sh
# Runs on the target rootfs (first arg) after the packages are installed.
set -e
TARGET="$1"

# We boot from an initramfs, so nothing mounts devtmpfs for us: without it /dev has no fb0/input nodes.
if ! grep -q 'devtmpfs devtmpfs /dev' "$TARGET/etc/inittab"; then
    sed -i '0,/^::sysinit:\/bin\/mkdir -p \/dev\/pts/s||::sysinit:/bin/mount -t devtmpfs devtmpfs /dev\n&|' "$TARGET/etc/inittab"
fi
# The graphical shell owns the screen: keep the login prompt on the serial port only.
sed -i 's|^console::respawn:/sbin/getty.*|ttyS0::respawn:/sbin/getty -L ttyS0 115200 vt100|' "$TARGET/etc/inittab"

chmod 0755 "$TARGET/etc/init.d/S45clock"

# Secrets stay out of the source tree: copied in at build time from the local, untracked .secrets/ folder.
mkdir -p "$TARGET/etc/tvos"
if [ -f "${TVOS_YT_KEY_FILE:-}" ]; then
    install -m 0600 "$TVOS_YT_KEY_FILE" "$TARGET/etc/tvos/youtube.key"
fi

# Start the shell as the only foreground program; restart it if it ever exits.
sed -i '/tvos-shell/d' "$TARGET/etc/inittab"
echo '::respawn:/usr/bin/tvos-shell >/dev/console 2>&1' >> "$TARGET/etc/inittab"

# Pick up the binary that build-vm.sh staged in the overlay's staging dir.
install -D -m 0755 "${TVOS_SHELL_BIN:?set TVOS_SHELL_BIN}" "$TARGET/usr/bin/tvos-shell"
