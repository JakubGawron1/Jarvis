# mkosi — Arch

Jarvis Linux is **Arch-based** (not Debian). Plasma is the DE; HUD overlay sits on top.

```bash
# from linux/distro/arch on an Arch host
sudo mkosi --force
# output disk → ../qemu/jarvis-linux.qcow2 (see mkosi.conf Output=)
```

Without mkosi, bootstrap a similar rootfs:

```bash
sudo pacstrap -K /mnt base linux linux-firmware systemd networkmanager \
  plasma-desktop sddm konsole dolphin chromium \
  pipewire pipewire-pulse noto-fonts sudo nodejs pnpm
```

Then copy `jarvisd` to `/usr/local/bin`, enable `jarvisd.service` + `sddm`, set `JARVIS_BOOT` from kernel cmdline `jarvis.boot=qemu|vbox|metal`.
