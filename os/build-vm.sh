#!/bin/bash
# Builds the TVos VM ISO inside WSL/Linux. Usage: os/build-vm.sh
# First run builds WPE WebKit and the kernel: expect 1-2 hours. Later runs only redo what changed.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BR="${BR:-$HOME/buildroot-2026.08.1}"
OUT="${OUT:-$HOME/br-out-vm2}"
# Buildroot refuses a PATH containing spaces (WSL inherits the Windows one)
export PATH="/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:$HOME/.cargo/bin"
# WebKit needs ~1.5 GB per compile job: with 8 GB of RAM keep the job count low
JOBS="${JOBS:-4}"

echo "[1/4] configuring Buildroot + toolchain"
make -C "$BR" O="$OUT" BR2_EXTERNAL="$ROOT/os" tvos_vm_x86_64_defconfig
make -C "$BR" O="$OUT" toolchain

echo "[2/4] building tvos-shell with the Buildroot toolchain (glibc, same as the Pi)"
CC="$OUT/host/bin/x86_64-linux-gcc"
export CC_x86_64_unknown_linux_gnu="$CC"
export CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER="$CC"
(cd "$ROOT" && CARGO_TARGET_DIR="$HOME/tvos-target" cargo build --release -p tvos-shell --target x86_64-unknown-linux-gnu)
export TVOS_SHELL_BIN="$HOME/tvos-target/x86_64-unknown-linux-gnu/release/tvos-shell"
export TVOS_YT_KEY_FILE="$ROOT/.secrets/youtube_api_key"

echo "[3/4] building the image"
make -C "$BR" O="$OUT" -j"$JOBS"

echo "[4/4] done"
ls -la "$OUT/images/"
