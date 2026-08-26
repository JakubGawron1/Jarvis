# Deploy — Render, aplikacje, Linux, stable build

Lista rzeczy, które **Ty** musisz zrobić, żeby rdzeń, HUD, Flutter i Linux działały razem, a build był powtarzalny. Nie commituj `.env`. Node wyłącznie przez **pnpm** (root ma `packageManager: pnpm@11.24.0`).

## Mapa

| Co | Gdzie | Rola |
|---|---|---|
| `jarvisd` | `jarvis/` | Rdzeń: WS `7420`, LLM, pamięć, mesh |
| HUD | `hud/` | Next.js 16 — szkło Iron Man (web + overlay Linuksa) |
| Flutter | `app/` | Windows / Android / Linux — ten sam protokół WS |
| Render | `cloud/` | Fallback `jarvisd` (Free, sen po 15 min) |
| Linux desktop | `linux/desktop.md` | Twój Arch / Jarvis Linux po pierwszym bootcie |
| Distro | `linux/distro/` | Obraz Arch + Plasma + overlay (QEMU / VBox / USB) |

Kolejność leadera meshu: **desktop (Win/Linux) → telefon → Render** (budzenie na żądanie).

---

## 1. Narzędzia (raz na maszynę)

**Windows (dev + QEMU)**

- [ ] Rust **1.85+** (`rustup update stable`) — workspace `edition = "2024"`
- [ ] Node **22+** + **Corepack** → `corepack enable` i `corepack use pnpm@11.24.0` w katalogu repo
- [ ] Flutter SDK z Dart **^3.13** (`flutter doctor`)
- [ ] Visual Studio Build Tools (Windows desktop) albo już działający `flutter run -d windows`
- [ ] Android SDK / emulator albo telefon z USB debugging (opcjonalnie)
- [ ] QEMU (`C:\Program Files\qemu\qemu-system-x86_64.exe` + `edk2-x86_64-code.fd`)
- [ ] WSL2 **Ubuntu** (`wsl --install -d Ubuntu`) — tylko do buildu distro
- [ ] Konto [Render](https://render.com), opcjonalnie [Vercel](https://vercel.com), [OpenRouter](https://openrouter.ai), [Turso](https://turso.tech)
- [ ] Tailscale na PC, telefonie i (po boocie) na Jarvis Linux — mesh LAN bez dziur w NAT

**Linux desktop (Arch / Jarvis Linux)**

- [ ] `rustup`, `pnpm` (Corepack), Chromium, PipeWire
- [ ] `cargo` na PATH jeśli chcesz `rewrite_core`

Lokalny LLM (Bionic / LM Studio) na `http://127.0.0.1:1234/v1` — **nie** idzie na Render. Cloud używa OpenRouter.

---

## 2. Sekrety — `.env`

```powershell
copy .env.example .env
```

Wypełnij (puste = dana ścieżka wyłączona / degradacja do tekstu):

| Zmienna | Po co |
|---|---|
| `JARVIS_LOCAL_LLM_URL` / `JARVIS_LOCAL_LLM_MODEL` | Bionic/LM Studio na desktopie |
| `OPENROUTER_API_KEY` / `OPENROUTER_MODEL` | fallback LLM (desktop + Render) |
| `OPENROUTER_TTS_MODEL` / `_EN` / `OPENROUTER_TTS_VOICE_EN` | darmowe TTS na Renderze: Fish (PL) + Flux `flux-sean-en` (EN). Ten sam klucz co LLM. |
| `JARVIS_PAIRING_TOKEN` | **obowiązkowy na Renderze**; lokalnie może być pusty |
| `JARVIS_BIND` | lokalnie zostaw `127.0.0.1:7420` |
| `TURSO_URL` / `TURSO_AUTH_TOKEN` | wspólna pamięć w chmurze (Render nie ma dysku) |
| `WHISPER_BIN`, `PIPER_*`, `KOKORO_URL` | głos neuralny; bez nich desktop i tak mówi (Windows SAPI / espeak-ng). `scripts/setup-voice.ps1` ściąga Piper. HUD odtwarza ramkę `speech`. |

Ten sam `JARVIS_PAIRING_TOKEN` musi być w Render **i** w klientach, które biją na `wss://…`. Każda ramka JSON na WS musi mieć `"token"` albo `"pairing_token"` — inaczej daemon odsyła `unauthorized`. HUD i Flutter **na razie wysyłają sam `type/content`**; do Rendera doklej token w wiadomości albo tymczasowo zostaw token pusty (wtedy URL jest publiczny — tylko do testów).

---

## 3. Stable build (brama przed deployem)

Z katalogu repo, w tej kolejności. Jak coś pada — nie idź na Render / distro.

### 3.1 Rdzeń

```powershell
cd jarvis
cargo test --workspace
cargo build --release -p jarvis-daemon -p jarvis-cli
```

Binarki: `jarvis\target\release\jarvisd.exe` i `jarvis-cli.exe`.

Smoke:

```powershell
cd ..
copy .env.example .env   # jeśli jeszcze nie
# w jednym terminalu:
.\jarvis\target\release\jarvisd.exe
# w drugim:
curl http://127.0.0.1:7420/health
```

Oczekuj JSON `{ "ok": true, "version": "…" }`.

### 3.2 HUD (web)

```powershell
pnpm install
pnpm --filter jarvis-hud build
```

Dev: `pnpm --filter jarvis-hud dev` → `http://127.0.0.1:3000`  
Overlay Linuksa: `http://127.0.0.1:3000/?overlay=1`  
Inny daemon: `http://127.0.0.1:3000/?ws=ws://127.0.0.1:7420/ws`

`hud/next.config.mjs` ma `output: "standalone"` — produkcja to `pnpm --filter jarvis-hud start` albo Vercel (szkło; WS i tak idzie do `jarvisd`).

### 3.3 Flutter

```powershell
cd app
flutter pub get
flutter analyze
flutter test
flutter build windows --release
# telefon:
flutter build apk --release
# Linux (na Arch / Jarvis Linux, nie na Windows):
flutter build linux --release
```

Domyślny WS: `ws://127.0.0.1:7420/ws`. Na Androidzie zmień na Tailscale PC (`ws://100.x.x.x:7420/ws`) albo Render `wss://<usługa>.onrender.com/ws`.

FFI (`libjarvis_ffi.so` / `jarvis_ffi.dll`) to osobny `cargo build --release -p jarvis-ffi` — telefon jako pełny mózg, gdy desktopów nie ma.

### 3.4 Obraz Dockera (ten sam co Render)

Z **roota** repo (context `.`, nie `cloud/`):

```powershell
docker build -f cloud/Dockerfile -t jarvis-core:local .
docker run --rm -p 7420:7420 -e JARVIS_BIND=0.0.0.0:7420 -e OPENROUTER_API_KEY -e JARVIS_PAIRING_TOKEN jarvis-core:local
curl http://127.0.0.1:7420/health
```

Dockerfile kopiuje `jarvis/`, `skills/`, `vault/`, `releases/`. Katalog `releases/` musi istnieć (`releases/manifest.json`).

---

## 4. Render (cloud `jarvisd`)

Blueprint: `render.yaml` w **rootcie** repo (Render czyta ten plik). Darmowy web, Docker, sen **po ~15 min bez ruchu**, cold start **~1 min** (`core_waking`). **Brak** tooli OS, **brak** `rewrite_core`, **brak** trwałego dysku.

### 4.1 Dashboard (albo Blueprint)

1. New → Blueprint / Web Service, root repo = monorepo.
2. Runtime: **Docker**
3. Dockerfile: `cloud/Dockerfile`
4. Context: `.` (root repo)
5. Health check: `/health`
6. Plan: **Free**

### 4.2 Env na Renderze (krytyczne)

Render nasłuchuje na **`PORT`** (zwykle **10000**), nie na 7420. Daemon bierze bind w kolejności: `JARVIS_BIND` → `PORT` → `0.0.0.0:7420` w trybie cloud. **Nie ustawiaj** `JARVIS_BIND` na Renderze — wtedy health check trafia w `PORT`. W panelu ustaw:

| Key | Wartość |
|---|---|
| `JARVIS_KIND` | `cloud` (Dockerfile i Blueprint już to ustawiają) |
| `JARVIS_ROOT` | `/app` (Dockerfile) |
| `OPENROUTER_API_KEY` | klucz free-tier |
| `OPENROUTER_MODEL` | np. `openrouter/auto` |
| `JARVIS_PAIRING_TOKEN` | wygeneruj (Blueprint ma `generateValue: true`) — **skopiuj do lokalnego `.env`** |
| `TURSO_URL` / `TURSO_AUTH_TOKEN` | jeśli chcesz pamięć po śnie instancji |

Nie ustawiaj `JARVIS_LOCAL_LLM_URL` na Renderze — tam nie ma Bionica.

### 4.3 Po deployu

```text
https://<nazwa>.onrender.com/health     →  budzi instancję
wss://<nazwa>.onrender.com/ws           →  sesja
```

1. Najpierw `GET /health` (przeglądarka albo `curl`) — poczekaj aż wróci `ok`.
2. Potem HUD/Flutter na `wss://…/ws` z tokenem w ramce.
3. Po sesji rozłącz WS — Free ma **750 godzin/mies.** instancji.

Nowy cloud core = zwykły `git push` (nie `--force`) → rebuild Dockera.

Alternatywa: ten sam Dockerfile na Hugging Face Spaces.

---

## 5. HUD w produkcji

**Wariant A — lokalnie / Linux (zalecane do overlayu)**  
`pnpm --filter jarvis-hud build` potem `pnpm --filter jarvis-hud start` (`PORT=3000`). Overlay: `linux/hud-overlay.sh` albo Super+J.

Jednostki user-systemd (istniejący Arch): skopiuj `linux/systemd/jarvisd.service` i `linux/systemd/jarvis-hud.service`, potem:

```bash
systemctl --user enable --now jarvisd.service jarvis-hud.service
```

HUD service robi `pnpm start` w `~/Jarvis/hud` — **najpierw** `pnpm install` + `pnpm build` w tym katalogu (albo z roota workspace).

**Wariant B — Vercel (tylko szkło)**  
Zaimportuj repo, root directory **`hud`**, framework Next, **pnpm**. Env nie musi mieć kluczy LLM. Klient i tak łączy się do `jarvisd` (`?ws=wss://….onrender.com/ws` albo Tailscale). Vercel nie hostuje daemona.

---

## 6. Flutter — instalacja u Ciebie

| Host | Komenda | WS |
|---|---|---|
| Windows | `flutter run -d windows` / `flutter build windows --release` | `ws://127.0.0.1:7420/ws` |
| Android | `flutter install` / APK | Tailscale PC albo Render `wss://` |
| Linux | `flutter run -d linux` | lokalny `jarvisd` |

Telefon bez desktopu: OpenRouter albo mały GGUF + `pull_core` / FFI. PTT to przycisk (One UI zabija always-on mic). Tekst zawsze działa.

---

## 7. Linux desktop (już zainstalowany Arch)

To **nie** jest obraz distro. Ten sam `jarvisd` co na Windows.

```bash
cd ~/Jarvis/jarvis && cargo run --release -p jarvis-daemon
# drugi terminal, root repo:
pnpm install && pnpm --filter jarvis-hud build && pnpm --filter jarvis-hud start
./linux/hud-overlay.sh
```

Plasma: **System Settings → Shortcuts → Custom Shortcuts** → Super+J → `~/Jarvis/linux/hud-overlay.sh`.  
Szczegóły: `linux/desktop.md`.

---

## 8. Jarvis Linux (obraz QEMU / VBox / metal)

Jeden rootfs: Arch + Plasma (zwykły pulpit) + overlay HUD. Build **z Windowsa przez WSL** (mkosi w sandboxie WSL nie ma DNS — skrypt idzie bootstrap/pacstrap).

### 8.1 Zbuduj obraz

```powershell
powershell -File linux\distro\build.ps1
```

Pierwszy raz: kilka GB (Arch bootstrap + Plasma + Rust). Wynik:

- `\\wsl.localhost\Ubuntu\home\jakub\jarvis-distro\jarvis-linux.qcow2`
- opcjonalnie kopia `linux\distro\qemu\jarvis-linux.qcow2` (jeśli na C: jest miejsce)

### 8.2 QEMU na Windows

```powershell
powershell -File linux\distro\qemu\run-qemu.ps1
```

Login: **jarvis** / **jarvis** (root to samo). Host forward: `localhost:7420` → gość `:7420`.

### 8.3 VirtualBox

`linux/distro/vbox/README.md` — `qemu-img convert -O vdi`, 2 CPU / 2 GB+, virtio-net.

### 8.4 Goły metal (USB)

`linux/distro/metal/README.md` — ISO/USB (Rufus albo `dd`). Kernel musi mieć virtio **i** AHCI/NVMe, USB HID, typowe NIC.

### 8.5 Po pierwszym boocie (obowiązkowe)

W obrazie jest `jarvisd` (systemd) i źródła HUD w `/opt/jarvis/hud`, **bez** `node_modules`. Overlay otwiera Chromium na `:3000` — HUD musisz postawić:

```bash
cd /opt/jarvis/hud
# z root workspace lepiej, ale na obrazie HUD jest pod /opt/jarvis/hud
corepack enable
pnpm install
pnpm build
pnpm start
```

Albo z hosta zsynchronizuj już zbudowany `.next` (ciężkie). Super+J na obrazie jest **niezbindowane** — dodaj skrót do `jarvis-hud-overlay` (albo menu: *Jarvis HUD Overlay*).

Kernel cmdline / env: `jarvis.boot=qemu|vbox|metal` → `JARVIS_BOOT`, żeby mesh **nie scalał** instancji QEMU z USB.

Tailscale na gościu, potem `device_hello` w HUD na Windows.

---

## 9. Żeby „wszystko gadało”

1. Desktop `jarvisd` na `127.0.0.1:7420` + lokalny LLM **albo** OpenRouter.
2. HUD `pnpm dev` / `start` → rozmowa PL/EN, hologramy (*pokaż model atomu*).
3. Flutter na PC: ten sam WS.
4. Telefon: Tailscale IP desktopu; gdy PC śpi — Render `wss://` + token + wcześniejszy ping `/health`.
5. Jarvis Linux w QEMU: Tailscale albo `hostfwd` na 7420; `JARVIS_BOOT=qemu`.
6. Ten sam `OPENROUTER_API_KEY` na desktopie i Renderze; Turso jeśli pamięć ma przeżyć sen Rendera.

---

## 10. Checklista stable release

Zaznacz zanim uznasz build za stabilny:

- [ ] `cargo test --workspace` i `cargo build --release -p jarvis-daemon`
- [ ] `pnpm --filter jarvis-hud build`
- [ ] `flutter analyze` + `flutter test`
- [ ] `GET http://127.0.0.1:7420/health` przy odpalonym `jarvisd`
- [ ] HUD łączy się lokalnie, widać Arc Reactor ONLINE
- [ ] Docker lokalnie: `/health` na zmapowanym porcie
- [ ] Render: `/health` po cold starcie (daemon słucha na `PORT`)
- [ ] Token Rendera skopiowany; bez tokenu w ramce nie ma sesji (gdy token jest ustawiony)
- [ ] Distro: QEMU boot, login, `jarvisd` w `systemctl`, HUD na `:3000`, overlay
- [ ] `.env` i klucze **nie** są w gicie

Głos, Turso i Vercel są opcjonalne. Bez nich: tekst + lokalna/OpenRouter pamięć sqlite na hoście, HUD i Flutter nadal działają.
