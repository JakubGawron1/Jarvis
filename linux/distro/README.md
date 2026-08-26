# Jarvis Linux (distro)

**Arch Linux** + **KDE Plasma** (normal desktop) + Jarvis HUD overlay. One rootfs, three boots.

| Target | Artifact | How |
|---|---|---|
| QEMU | `qemu/jarvis-linux.qcow2` or WSL `~/jarvis-distro/` | `qemu/run-qemu.ps1` / `.sh` |
| VirtualBox | convert qcow2 → VDI | see `vbox/` |
| Real PC | ISO / USB | see `metal/` |

## Build on Windows

You already have **WSL2 Ubuntu** and **QEMU**. The image is baked in WSL (ext4) from the official Arch bootstrap + `pacstrap` (mkosi's sandbox cannot resolve DNS under WSL).

```powershell
powershell -File linux\distro\build.ps1
```

That will:

1. Installs build tools and Rust in Ubuntu WSL (first time).
2. Builds `jarvisd` for Linux.
3. Downloads Arch bootstrap and `pacstrap`s Plasma + Jarvis (first run: several GB).
4. Writes `\\wsl.localhost\Ubuntu\home\jakub\jarvis-distro\jarvis-linux.qcow2`.

Boot:

```powershell
powershell -File linux\distro\qemu\run-qemu.ps1
```

Login: **jarvis** / **jarvis** (root same password). Super+J is unbound until you add a Plasma shortcut to `jarvis-hud-overlay`.

Set `JARVIS_BOOT=qemu|vbox|metal` via kernel cmdline `jarvis.boot=…` so mesh `device_hello` does not merge instances.
