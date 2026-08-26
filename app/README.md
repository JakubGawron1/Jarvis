# Jarvis Flutter

Windows, Android, Linux. Talks JSON WebSocket to `jarvisd` (same protocol as the HUD).

```bash
flutter pub get
flutter run -d windows
```

On Android, set the WS URL to the PC Tailscale IP (`ws://100.x.x.x:7420/ws`) or Render `wss://…/ws`.

Native core: `lib/jarvis_ffi.dart` loads `jarvis-ffi` after `pull_core` ships a new `.so`. Phone is a full brain when desktops are off (OpenRouter / small GGUF).

PTT is a button (One UI kills always-on mic). Text field is always visible.
