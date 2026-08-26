# Pull core (every host)

On start, `jarvisd` can load `releases/manifest.json`. Clients send `{ "type": "pull_core" }`.

Flutter: after download, swap `libjarvis_ffi.so` / `jarvis_ffi.dll` and restart.

Render: prefer git push → Docker rebuild rather than mutating the container disk (ephemeral).
