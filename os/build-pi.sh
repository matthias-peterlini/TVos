#!/bin/bash
# Builds the Raspberry Pi 3B SD-card image inside WSL/Linux. Usage: os/build-pi.sh
# First run downloads the toolchain and the Raspberry Pi kernel and takes 40-60 minutes.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BR="${BR:-$HOME/buildroot-2026.08.1}"
OUT="${OUT:-$HOME/br-out-pi}"
# Buildroot refuses a PATH containing spaces (WSL inherits the Windows one)
export PATH="/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:$HOME/.cargo/bin"

echo "[1/4] configuring Buildroot + toolchain"
make -C "$BR" O="$OUT" BR2_EXTERNAL="$ROOT/os" tvos_rpi3_defconfig
make -C "$BR" O="$OUT" toolchain

echo "[2/4] building tvos-shell for aarch64 with the Buildroot toolchain"
rustup target add aarch64-unknown-linux-gnu >/dev/null
CC="$OUT/host/bin/aarch64-linux-gcc"
export CC_aarch64_unknown_linux_gnu="$CC"
export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER="$CC"
(cd "$ROOT" && CARGO_TARGET_DIR="$HOME/tvos-target" cargo build --release -p tvos-shell --target aarch64-unknown-linux-gnu)
export TVOS_SHELL_BIN="$HOME/tvos-target/aarch64-unknown-linux-gnu/release/tvos-shell"
export TVOS_YT_KEY_FILE="$ROOT/.secrets/youtube_api_key"
export TVOS_WIFI_FILE="$ROOT/.secrets/wifi.conf"

echo "[3/4] building the image"
# BR2_JLEVEL also caps the jobs inside each package (WebKit's ninja would otherwise use every core: ~1.5 GB each)
make -C "$BR" O="$OUT" -j"${JOBS:-4}" BR2_JLEVEL="${JOBS:-4}"

echo "[4/4] done"
mkdir -p "$ROOT/out"
cp "$OUT/images/sdcard.img" "$ROOT/out/tvos-rpi3.img"
ls -la "$ROOT/out/tvos-rpi3.img"
