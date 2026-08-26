# Jarvis HUD

Next.js Arc Reactor: CPU/RAM, model, device list, text chat (PL/EN), holographic visuals.

```bash
pnpm install
pnpm --filter jarvis-hud dev
```

Connects to `ws://127.0.0.1:7420/ws`. Override with `?ws=` or the endpoint field.

## Overlay (Linux desktop)

`http://127.0.0.1:3000/?overlay=1` — transparent glass + amber hologram over Plasma. Plasma remains the real desktop. Toggle with `linux/hud-overlay.sh` (Super+J).

Ask for **anything** visual: *pokaż model atomu*, *show DNA*, *prezentacja o X*, *wykres Y*. The core builds a `Visual` spec (3D scene, slides, diagram, or procedural clip); the HUD renders it in WebGL.

Vercel hosts only the glass. Pairing token: put it in localStorage later; for Render pass `token` on each JSON frame.

PTT button currently sends text (browser mic STT can be wired to Whisper later).
