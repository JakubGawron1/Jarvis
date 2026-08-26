#!/usr/bin/env bash
# Boot Jarvis Linux (Arch + Plasma) in QEMU (virtio).
set -euo pipefail
DISK="${1:-$(dirname "$0")/jarvis-linux.qcow2}"
if [[ ! -f "$DISK" ]]; then
  echo "Missing $DISK — build the rootfs first (see ../README.md)"
  echo "Creating a placeholder 4G qcow2 so the command is documented."
  qemu-img create -f qcow2 "$DISK" 4G
fi
exec qemu-system-x86_64 \
  -m 2048 \
  -smp 2 \
  -drive file="$DISK",if=virtio,format=qcow2 \
  -netdev user,id=n0,hostfwd=tcp::7420-:7420 \
  -device virtio-net-pci,netdev=n0 \
  -nographic
