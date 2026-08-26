# Architecture

Jarvis is a **multi-host agent**. There is no single brain in the cloud.

## Hosts

1. `jarvisd` on Windows and Linux desktop (preferred leader; Bionic, OS tools, `rewrite_core` if cargo is present).
2. Flutter FFI core on Android (leader when desktops are off).
3. Render Free Docker `jarvisd` (on-demand; OpenRouter + Turso; no OS tools).
4. Jarvis Linux distro (phase 7): same `jarvisd` as system commander, bootable in QEMU, VirtualBox, and on a real PC.

## Protocol

JSON over WebSocket (HUD, cloud) or FFI (Flutter). Client sends `text` or `utterance`; the agent loop is the same. See [`PROTOCOL.md`](PROTOCOL.md).

## Language

Understand Polish and English. Reply in the language of the last user turn (mixed → PL). TTS: Piper `pl_PL` / Kokoro `bm_george` (EN).

## Memory

Local SQLite cache + optional Turso as shared source of truth. Vault notes live in `vault/` (git).

## Updates

- `rewrite_core`: desktop with cargo only (worktree, test, publish).
- `pull_core`: every host checks `releases/manifest.json` and swaps binaries (hash + last-known-good).
