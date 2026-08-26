# Cloud fallback (Render Free)

Same `jarvisd`, host mode cloud: **no OS tools, no rewrite_core**.

Blueprint for Render: [`/render.yaml`](../render.yaml) (repo root — that is what Render reads). Docker build: this folder's `Dockerfile`, context is the monorepo root.

- Sleep after 15 minutes idle; cold start ~1 minute (`core_waking`).
- 750 instance hours/month — connect on demand, disconnect after the session.
- No persistent disk — memory via Turso (`TURSO_URL`).
- Pair with `JARVIS_PAIRING_TOKEN`.
- TTS: OpenRouter free speech (`OPENROUTER_API_KEY`). Polish → `fish-audio/s2.1-pro-free:free`. English → `deepgram/flux-tts:free` (`flux-sean-en`, British male). HUD/Flutter play the `speech` frame (mp3).
- Alternate: same Docker on Hugging Face Spaces.

Ping `GET /health` to wake, then `wss://…/ws`.
