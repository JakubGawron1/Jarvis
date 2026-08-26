# Jarvis

Personal assistant: one Rust core, many hosts. Text and voice (PL + EN). Replies in the user's language (Polish → Piper, English → Kokoro).

[![Deploy to Render](https://render.com/images/deploy-to-render-button.svg)](https://render.com/deploy?repo=https://github.com/JakubGawron1/Jarvis)

## Map

| Path | What |
|---|---|
| [`jarvis/`](jarvis/) | Core workspace (`jarvisd`, CLI, protocol, tools, mesh, voice, update) |
| [`hud/`](hud/) | Next.js Arc Reactor HUD |
| [`app/`](app/) | Flutter (Windows / Android / Linux) |
| [`cloud/`](cloud/) | Render Free fallback (`Dockerfile`) |
| [`linux/`](linux/) | Arch desktop notes + Jarvis Linux (Plasma + HUD overlay; QEMU / VirtualBox / metal) |
| [`os/`](os/) | JarvisOS kernel research (not bootable yet) |
| [`skills/`](skills/) | Persona and agent skills (git) |
| [`vault/`](vault/) | Markdown notes (git, offline) |

## Run (Windows / Linux desktop)

1. Copy `.env.example` → `.env`. Point `JARVIS_LOCAL_LLM_URL` at Bionic/LM Studio (`http://127.0.0.1:1234/v1`) or set `OPENROUTER_API_KEY`.
2. Core:

```bash
cd jarvis
cargo run -p jarvis-daemon
```

3. CLI (another terminal):

```bash
cargo run -p jarvis-cli -- "jaki mam kalendarz"
```

4. HUD:

```bash
pnpm install
pnpm --filter jarvis-hud dev
```

Or from `hud/`: `pnpm dev`.

Default WebSocket: `ws://127.0.0.1:7420/ws`. Overlay over a normal Linux desktop: `http://127.0.0.1:3000/?overlay=1` (see `linux/desktop.md`). Ask the HUD to show a model, slides, or a chart — holograms are generic, not a single demo.

## Leader order

Desktop (Windows or Linux) → phone → Render (wake on demand). Live handoff of I/O and compute during a session. Jarvis Linux (QEMU / VBox / USB) joins the mesh as its own device.

## Cloud (Render)

Fallback `jarvisd` (Free, sleeps after ~15 min). Blueprint is [`render.yaml`](render.yaml) at the repo root — Docker image from [`cloud/Dockerfile`](cloud/Dockerfile). HUD is not deployed there (local or Vercel).

After connecting the GitHub repo on Render, set `OPENROUTER_API_KEY` in the dashboard (Blueprint leaves it empty on purpose). Copy the generated `JARVIS_PAIRING_TOKEN` into local `.env`.

```text
https://<service>.onrender.com/health
wss://<service>.onrender.com/ws
```

## License

Personal project. Source is public.
