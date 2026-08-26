# Voice

Text works without this crate's binaries. If Piper and Kokoro are unset, desktop uses **OpenRouter free TTS** (same `OPENROUTER_API_KEY`). SAPI / espeak-ng only when that key is missing. **Render** is OpenRouter only.

## STT

[whisper.cpp](https://github.com/ggerganov/whisper.cpp) — multilingual `small` on desktop, `tiny`/`base` on phone.

```
WHISPER_BIN=path/to/whisper-cli
```

## TTS

**Cloud / Render** (`JARVIS_KIND=cloud`):

| Lang | Model (free) | Voice |
|---|---|---|
| PL | `fish-audio/s2.1-pro-free:free` | auto (83 languages, including Polish) |
| EN | `deepgram/flux-tts:free` | `flux-sean-en` (British male) |

Override with `OPENROUTER_TTS_MODEL`, `OPENROUTER_TTS_MODEL_EN`, `OPENROUTER_TTS_VOICE_EN`. If Flux fails, Fish is the fallback. HUD plays `speech` as `audio/mpeg`.

**Desktop** (no Piper / no Kokoro → OpenRouter, same key as the LLM):

| Lang | 1 | 2 | 3 |
|---|---|---|---|
| PL | Piper if `PIPER_BIN` + voice | **OpenRouter Fish** | SAPI only if no API key |
| EN | Kokoro if `KOKORO_URL`, else Piper | **OpenRouter Flux** (Fish fallback) | SAPI only if no API key |

Windows one-shot (Piper + PL/EN voices into `vendor/piper`):

```powershell
powershell -File scripts\setup-voice.ps1
```

Wake word: openWakeWord “jarvis” / “dżarvis” on desktop only. Phone: push-to-talk.
