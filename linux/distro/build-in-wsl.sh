#!/usr/bin/env bash
# Build Jarvis Linux (Arch + Plasma) inside WSL. Invoked as root from build.ps1.
set -euo pipefail

export HOME="${HOME:-/root}"
export PATH="/root/.local/bin:/root/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"
if [ "$(id -u)" -ne 0 ]; then
  echo "Run as root: wsl -d Ubuntu -u root ..." >&2
  exit 1
fi

WIN_REPO="${1:-/mnt/c/Users/jakub/Desktop/Jarvis}"
OUT_HOME=/home/jakub
WORK="$OUT_HOME/jarvis-distro"
SRC="$WORK/src"
ARCH="$SRC/linux/distro/arch"

echo "==> Jarvis Linux build (WSL root)"
echo "    source: $WIN_REPO"
echo "    work:   $WORK"

mkdir -p "$WORK" "$OUT_HOME"
find "$WIN_REPO/linux/distro" -type f \( -name '*.sh' -o -name '*.chroot' -o -name 'write-bootenv' -o -name 'jarvis-hud-overlay' \) \
  -exec sed -i 's/\r$//' {} \; 2>/dev/null || true

echo "==> packages"
apt-get update -qq
DEBIAN_FRONTEND=noninteractive apt-get install -y -qq \
  python3 python3-venv python3-pip pipx \
  qemu-utils uidmap dbus bubblewrap \
  systemd-container ca-certificates curl git rsync \
  attr acl unzip debianutils debian-archive-keyring ubuntu-keyring

if ! command -v cargo >/dev/null 2>&1; then
  echo "==> rustup"
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
fi
# shellcheck disable=SC1091
source /root/.cargo/env 2>/dev/null || export PATH="/root/.cargo/bin:$PATH"

echo "==> sync tree to ext4"
mkdir -p "$SRC"
rsync -a --delete \
  --exclude '/jarvis/target' \
  --exclude '/hud/node_modules' \
  --exclude '/hud/.next' \
  --exclude '/app/build' \
  --exclude '/app/.dart_tool' \
  --exclude '/node_modules' \
  --exclude '/linux/distro/**/*.qcow2' \
  --exclude '/linux/distro/**/*.raw' \
  --exclude '/.git' \
  "$WIN_REPO/" "$SRC/"

echo "==> jarvisd (Linux)"
cd "$SRC/jarvis"
cargo build --release -p jarvis-daemon
install -D -m 0755 target/release/jarvisd "$ARCH/mkosi.extra/usr/local/bin/jarvisd"

echo "==> /opt/jarvis payload"
mkdir -p "$ARCH/mkosi.extra/opt/jarvis"
rsync -a --delete \
  --exclude node_modules --exclude .next \
  "$SRC/skills" "$SRC/vault" "$SRC/releases" "$SRC/hud" \
  "$ARCH/mkosi.extra/opt/jarvis/"
chmod 755 "$ARCH/mkosi.postinst.chroot" \
  "$ARCH/mkosi.extra/usr/local/bin/jarvis-hud-overlay" \
  "$ARCH/mkosi.extra/usr/local/lib/jarvis/write-bootenv"

echo "==> Arch disk (bootstrap + pacstrap)"
RAW="$WORK/jarvis-linux.raw"
sed -i 's/\r$//' "$SRC/linux/distro/bootstrap-disk.sh"
chmod +x "$SRC/linux/distro/bootstrap-disk.sh"
bash "$SRC/linux/distro/bootstrap-disk.sh" "$SRC" "$RAW"
IMG="$RAW"

QCOW="$WORK/jarvis-linux.qcow2"
echo "==> qcow2 $QCOW"
qemu-img convert -p -O qcow2 -c "$IMG" "$QCOW"
qemu-img info "$QCOW"
chown -R jakub:jakub "$WORK" || true

WIN_QEMU="$WIN_REPO/linux/distro/qemu/jarvis-linux.qcow2"
avail_kb=$(df -k /mnt/c | awk 'NR==2 {print $4}')
need_kb=$(du -k "$QCOW" | awk '{print $1}')
if [ "$avail_kb" -gt $((need_kb + 2*1024*1024)) ]; then
  echo "==> copy to Windows repo"
  mkdir -p "$(dirname "$WIN_QEMU")"
  cp -v "$QCOW" "$WIN_QEMU"
else
  echo "==> skip copy to C: (image is $((need_kb/1024)) MB, C: has $((avail_kb/1024)) MB free)"
  echo "    boot: \\\\wsl.localhost\\Ubuntu\\home\\jakub\\jarvis-distro\\jarvis-linux.qcow2"
fi

echo "==> done  $QCOW"
