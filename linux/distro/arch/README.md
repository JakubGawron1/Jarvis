# Arch image (mkosi)

Same rootfs for QEMU, VirtualBox, and metal. Plasma is the daily desktop; Jarvis HUD is an overlay, not a replacement DE.

```bash
sudo pacman -S mkosi arch-install-scripts
sudo mkosi --directory "$(dirname "$0")" --force
```

`mkosi.conf` writes a disk image. Convert/copy to `../qemu/jarvis-linux.qcow2` if the output filename differs.

## After first boot

1. SDDM → Plasma session (normal wallpaper, panel, windows).
2. `jarvisd` on `:7420` (systemd).
3. Super+J or `jarvis-hud-overlay.desktop` → HUD at `http://127.0.0.1:3000/?overlay=1`.
4. Ask: *pokaż model atomu* — hologram projects over the desktop.

Packages: see `packages.txt`.
