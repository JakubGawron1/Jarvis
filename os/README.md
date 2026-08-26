# JarvisOS

Research track: a Rust kernel where Jarvis is init-policy.

This is **not** Jarvis Linux (`linux/distro`). No QEMU bootable image in current phases.

ADR:

- Userspace protocol stays `jarvis-protocol` JSON.
- When a kernel exists, `jarvisd` becomes the policy process, not PID 1 on day one.
