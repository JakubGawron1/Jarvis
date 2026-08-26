#!/usr/bin/env bash
# Bake an Arch+Plasma GPT disk using the official bootstrap tarball (no mkosi sandbox).
set -euo pipefail
export PATH="/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"

SRC="${1:?src tree}"
OUT_RAW="${2:?output .raw path}"
ARCH="$SRC/linux/distro/arch"
WORK="$(dirname "$OUT_RAW")"
BOOTSTRAP="$WORK/arch-bootstrap"
TARGET="$WORK/mnt"
MIRROR="${JARVIS_ARCH_MIRROR:-https://geo.mirror.pkgbuild.com}"
ISO_BASE="$MIRROR/iso/latest"
TARBALL="archlinux-bootstrap-x86_64.tar.zst"

echo "==> bootstrap disk"
mkdir -p "$WORK" "$TARGET"
printf 'nameserver 1.1.1.1\nnameserver 8.8.8.8\n' >/etc/resolv.conf
umount -R "$TARGET" 2>/dev/null || true
if [ -f "$OUT_RAW" ]; then
  losetup -j "$OUT_RAW" | cut -d: -f1 | while read -r l; do
    [ -n "$l" ] && losetup -d "$l" || true
  done
fi

apt-get install -y -qq gdisk dosfstools e2fsprogs zstd curl systemd-container

if [ ! -f "$WORK/$TARBALL" ]; then
  echo "==> download $TARBALL"
  curl -fL --retry 5 -o "$WORK/$TARBALL" "$ISO_BASE/$TARBALL"
fi

if [ ! -d "$BOOTSTRAP/root.x86_64" ]; then
  echo "==> extract bootstrap"
  rm -rf "$BOOTSTRAP"
  mkdir -p "$BOOTSTRAP"
  tar --zstd -C "$BOOTSTRAP" -xf "$WORK/$TARBALL"
fi

echo "==> partition 16G GPT"
rm -f "$OUT_RAW"
truncate -s 16G "$OUT_RAW"
sgdisk -Z "$OUT_RAW"
sgdisk -n 1:0:+1G -t 1:ef00 -c 1:ESP "$OUT_RAW"
sgdisk -n 2:0:0 -t 2:8304 -c 2:root "$OUT_RAW"

LOOP=$(losetup --find --show --partscan "$OUT_RAW")
# WSL sometimes needs a moment for partitions.
for _ in 1 2 3 4 5; do
  [ -b "${LOOP}p2" ] && break
  sleep 1
  partx -u "$LOOP" 2>/dev/null || true
done
if [ ! -b "${LOOP}p2" ]; then
  echo "loop partitions missing on $LOOP" >&2
  ls -l "$LOOP"* >&2 || true
  losetup -d "$LOOP" || true
  exit 1
fi

cleanup() {
  umount -R "$TARGET" 2>/dev/null || true
  losetup -d "$LOOP" 2>/dev/null || true
}
trap cleanup EXIT

mkfs.vfat -F32 -n ESP "${LOOP}p1"
mkfs.ext4 -F -L jarvis-root "${LOOP}p2"
mount "${LOOP}p2" "$TARGET"
mkdir -p "$TARGET/boot"
mount "${LOOP}p1" "$TARGET/boot"

ROOT_UUID=$(blkid -s PARTUUID -o value "${LOOP}p2")
echo "    PARTUUID=$ROOT_UUID"

HOST="$BOOTSTRAP/root.x86_64"
cp /etc/resolv.conf "$HOST/etc/resolv.conf"
# Prefer a live mirror; bootstrap mirrorlist is often all-commented.
printf 'Server = %s/$repo/os/$arch\n' "$MIRROR" >"$HOST/etc/pacman.d/mirrorlist"
# Pacman in a bootstrap tree cannot see WSL mount "free space" and aborts.
sed -i 's/^CheckSpace/#CheckSpace/' "$HOST/etc/pacman.conf" || true
mkdir -p "$HOST/var/cache/pacman/pkg"

mount --bind "$TARGET" "$HOST/mnt"
mkdir -p "$HOST/mnt/boot"
mount --bind "$TARGET/boot" "$HOST/mnt/boot"
mount --bind /proc "$HOST/proc"
mount --bind /sys "$HOST/sys"
mount --bind /dev "$HOST/dev"
mount --bind /run "$HOST/run"
if [ -d /dev/pts ]; then
  mkdir -p "$HOST/dev/pts"
  mount --bind /dev/pts "$HOST/dev/pts"
fi

PKGS="base linux linux-firmware systemd networkmanager plasma-desktop plasma-workspace sddm konsole dolphin chromium nodejs pnpm pipewire pipewire-pulse wireplumber noto-fonts sudo git vim qemu-guest-agent wmctrl"

echo "==> pacman keys + pacstrap (Plasma, several GB)"
chroot "$HOST" /bin/bash -euo pipefail -c "
  pacman-key --init
  pacman-key --populate archlinux
  pacman --noconfirm --needed -Sy arch-install-scripts
  pacstrap -K /mnt $PKGS
"

echo "==> fstab, bootloader, Jarvis"
chroot "$HOST" /bin/bash -euo pipefail -c "genfstab -U /mnt >> /mnt/etc/fstab"

install -D -m 0755 "$ARCH/mkosi.extra/usr/local/bin/jarvisd" "$TARGET/usr/local/bin/jarvisd"
rsync -a "$ARCH/mkosi.extra/" "$TARGET/"
chmod 755 "$TARGET/usr/local/bin/jarvis-hud-overlay" "$TARGET/usr/local/lib/jarvis/write-bootenv" || true
chmod 440 "$TARGET/etc/sudoers.d/jarvis" || true

mkdir -p "$TARGET/boot/loader/entries"
cat >"$TARGET/boot/loader/loader.conf" <<'EOF'
default jarvis.conf
timeout 3
console-mode keep
EOF
cat >"$TARGET/boot/loader/entries/jarvis.conf" <<EOF
title Jarvis Linux
linux /vmlinuz-linux
initrd /initramfs-linux.img
options root=PARTUUID=${ROOT_UUID} rw quiet jarvis.boot=qemu
EOF

chroot "$HOST" /bin/bash -euo pipefail -c '
  arch-chroot /mnt /bin/bash -euo pipefail -c "
    ln -sf /usr/share/zoneinfo/Europe/Warsaw /etc/localtime
    echo jarvis-linux >/etc/hostname
    grep -q en_US.UTF-8 /etc/locale.gen || echo en_US.UTF-8 UTF-8 >>/etc/locale.gen
    grep -q pl_PL.UTF-8 /etc/locale.gen || echo pl_PL.UTF-8 UTF-8 >>/etc/locale.gen
    locale-gen
    echo LANG=en_US.UTF-8 >/etc/locale.conf
    bootctl --esp-path=/boot install
    groupadd -f wheel
    id jarvis >/dev/null 2>&1 || useradd -m -s /bin/bash -G wheel,video,audio,input jarvis
    echo jarvis:jarvis | chpasswd
    echo root:jarvis | chpasswd
    mkdir -p /opt/jarvis /etc/jarvis
    chown -R jarvis:jarvis /opt/jarvis
    systemctl set-default graphical.target
    systemctl enable NetworkManager sddm jarvisd jarvis-bootenv qemu-guest-agent
  "
'

chmod 644 "$TARGET/etc/systemd/system/"*.service 2>/dev/null || true

sync
sleep 1
umount -l "$HOST/mnt/boot" 2>/dev/null || true
umount -l "$HOST/mnt" 2>/dev/null || true
umount -l "$HOST/dev/pts" 2>/dev/null || true
umount -l "$HOST/dev" 2>/dev/null || true
umount -l "$HOST/run" 2>/dev/null || true
umount -l "$HOST/proc" 2>/dev/null || true
umount -l "$HOST/sys" 2>/dev/null || true
umount -l "$TARGET/boot" 2>/dev/null || true
umount -l "$TARGET" 2>/dev/null || true
sleep 1
trap - EXIT
losetup -d "$LOOP" 2>/dev/null || true

echo "==> raw disk ready $OUT_RAW"
