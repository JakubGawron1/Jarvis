# Bare metal

Arch-based hybrid ISO (UEFI + BIOS) written with Rufus (Windows) or:

```bash
dd if=jarvis-linux.iso of=/dev/sdX bs=4M status=progress conv=fsync
```

Kernel config must include virtio **and** AHCI/NVMe, USB HID, USB storage, common NICs.

Set `JARVIS_BOOT=metal`. After boot, SDDM → Plasma (normal desktop). Super+J opens the HUD overlay. `jarvisd` should send `device_hello` on the mesh (Tailscale or LAN).
