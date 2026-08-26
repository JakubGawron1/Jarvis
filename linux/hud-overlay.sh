#!/usr/bin/env bash
# Toggle Jarvis HUD overlay on the existing desktop (Plasma / any compositor).
set -euo pipefail
URL="${JARVIS_HUD_URL:-http://127.0.0.1:3000/?overlay=1}"
CLASS="jarvis-hud-overlay"

if command -v wmctrl >/dev/null 2>&1; then
  if wmctrl -lx | grep -qi "$CLASS\|overlay=1"; then
    pkill -f "overlay=1" || true
    exit 0
  fi
fi

launch() {
  if command -v chromium >/dev/null 2>&1; then
    exec chromium --app="$URL" --class="$CLASS" --ozone-platform=wayland \
      --enable-transparent-visuals --default-background-color=00000000 "$@"
  fi
  if command -v chromium-browser >/dev/null 2>&1; then
    exec chromium-browser --app="$URL" --class="$CLASS" "$@"
  fi
  if command -v google-chrome >/dev/null 2>&1; then
    exec google-chrome --app="$URL" --class="$CLASS" "$@"
  fi
  if command -v firefox >/dev/null 2>&1; then
    exec firefox --kiosk "$URL"
  fi
  echo "Install chromium or firefox" >&2
  exit 1
}

launch
