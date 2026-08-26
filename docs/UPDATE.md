# Desktop rewrite / publish

`rewrite_core` only on Windows or Linux with cargo:

```bash
cd jarvis
cargo run -p jarvis-cli -- rewrite --test-only
```

On success: commit, `git push` (Render rebuilds), cross-build:

```bash
cargo build -p jarvis-daemon --release
cargo build -p jarvis-ffi --release
# android: cargo ndk -t arm64-v8a build -p jarvis-ffi --release
```

Update `releases/manifest.json` with sha256. Hosts `pull_core` on start.

Last-known-good: keep previous binaries under `data/releases/`.
