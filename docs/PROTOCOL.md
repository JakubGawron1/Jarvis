# Protocol

All frames are JSON objects with a `type` field.

## Client → server

- `text` — `{ id, content, lang? }`
- `utterance` — `{ id, transcript?, audio_b64? }`
- `confirm` — `{ id, accepted }`
- `hello` — `{ device }` device caps / identity
- `handoff_request` — `{ target_device }`
- `pull_core` — `{}`
- `dismiss_visual` — `{}`

## Server → client

- `reply` — `{ id, content, lang }`
- `speech` — `{ id, mime, audio_b64 }` WAV after a reply (desktop TTS). Text still arrives if TTS is missing.
- `visual` — `{ id, spec, lang }` hologram / slides / diagram / clip (`VisualSpec`)
- `confirm` — `{ id, prompt, lang }`
- `task_progress` — `{ job_id, status, detail }`
- `device_hello` / `device_lost` / `presence`
- `handoff_ready` — `{ snapshot }`
- `job_deferred` — `{ job_id, until, message }`
- `core_waking` / `core_update`
- `stats` — CPU/RAM/model for Arc Reactor
- `error` — `{ message }`

`lang` is `pl` or `en`.
