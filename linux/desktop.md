# Linux desktop (Arch)

Run the same `jarvisd` as on Windows. This path is for an **existing Arch** install (or Jarvis Linux after first boot). Plasma stays the normal desktop; the Jarvis HUD is an optional holographic overlay.

```bash
cd jarvis
cargo run -p jarvis-daemon
```

In another terminal, from the repo root:

```bash
pnpm install
pnpm --filter jarvis-hud dev
```

## HUD as overlay, Plasma as desktop

Plasma (or any compositor) is the real desktop: wallpaper, panels, windows. The HUD is a second layer — amber hologram + glass panels — opened with **Super+J**.

```bash
# from repo root, after jarvisd + HUD are up
./linux/hud-overlay.sh
```

Opens Chromium/Firefox in app mode at `http://127.0.0.1:3000/?overlay=1`.

Bind Super+J in Plasma: **System Settings → Shortcuts → Custom Shortcuts** → command `~/Jarvis/linux/hud-overlay.sh` (toggle: running instance is focused or killed).

Install a systemd user unit for the daemon (optional):

```ini
# ~/.config/systemd/user/jarvisd.service
[Service]
ExecStart=/usr/local/bin/jarvisd
Environment=JARVIS_ROOT=%h/Jarvis
Restart=on-failure

[Install]
WantedBy=default.target
```

Ready-made units live in `linux/systemd/`. Copy and `systemctl --user enable --now jarvisd.service`.

Wake word is OK on a plugged-in desktop. `rewrite_core` works if `cargo` is on PATH. Tailscale recommended for mesh with Windows and the phone.

This is **not** Jarvis Linux (the distro). That lives in `distro/` and is **Arch-based**.
