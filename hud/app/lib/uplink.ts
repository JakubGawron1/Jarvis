export const LOCAL_WS = "ws://127.0.0.1:7420/ws";
export const RENDER_WS = "wss://jarvis-core-n12s.onrender.com/ws";
export const PAIRING_TOKEN = "uMrUM1mJIQFOmGPwMVekLpsjBTwV9QcO1lsX/im7l5I=";

function parseWs(url: string): URL | null {
  try {
    const raw = url.trim();
    if (!raw) return null;
    const u = new URL(raw);
    if (u.protocol === "http:") u.protocol = "ws:";
    if (u.protocol === "https:") u.protocol = "wss:";
    if (u.protocol !== "ws:" && u.protocol !== "wss:") return null;
    u.pathname = u.pathname.replace(/\/$/, "") || "/";
    if (u.pathname === "/") u.pathname = "/ws";
    else if (!u.pathname.endsWith("/ws")) u.pathname += "/ws";
    u.hash = "";
    u.search = "";
    return u;
  } catch {
    return null;
  }
}

export function normalizeWs(url: string): string {
  const u = parseWs(url);
  return u ? u.toString().replace(/\/$/, "") : "";
}

export function httpOriginFromWs(wsUrl: string): string {
  const u = parseWs(wsUrl) ?? parseWs(LOCAL_WS);
  if (!u) return "http://127.0.0.1:7420";
  u.protocol = u.protocol === "wss:" ? "https:" : "http:";
  return u.origin;
}

export function healthUrl(wsUrl: string): string {
  return `${httpOriginFromWs(wsUrl)}/health`;
}

export function statsUrl(wsUrl: string): string {
  return `${httpOriginFromWs(wsUrl)}/stats`;
}

export function isCloudFallbackHost(wsUrl: string): boolean {
  const u = parseWs(wsUrl);
  if (!u) return false;
  const host = u.hostname.toLowerCase();
  return (
    host.endsWith(".onrender.com") ||
    host.endsWith(".hf.space") ||
    host.endsWith(".huggingface.co")
  );
}

export function cloudWs(): string {
  const env = process.env.NEXT_PUBLIC_JARVIS_CLOUD_WS || "";
  return normalizeWs(env) || RENDER_WS;
}

export function uplinkLabel(wsUrl: string): string {
  if (!wsUrl) return "—";
  return isCloudFallbackHost(wsUrl) ? "Render" : "Local";
}

export function pairingToken(): string {
  const env = process.env.NEXT_PUBLIC_JARVIS_PAIRING_TOKEN || "";
  if (env) return env;
  return PAIRING_TOKEN;
}

export function withToken<T extends Record<string, unknown>>(body: T): T & { token?: string } {
  const token = pairingToken();
  return token ? { ...body, token } : body;
}

export function kickHealth(wsUrl: string) {
  const n = normalizeWs(wsUrl) || (isCloudFallbackHost(wsUrl) ? cloudWs() : LOCAL_WS);
  if (isCloudFallbackHost(n) && typeof window !== "undefined") {
    void fetch(`/api/uplink/health?ws=${encodeURIComponent(n)}`, { cache: "no-store" }).catch(() => {});
  }
  void fetch(healthUrl(n), { cache: "no-store", mode: "cors" }).catch(() => {});
}

export async function pingHealth(
  wsUrl: string,
  opts: { signal?: AbortSignal; timeoutMs?: number } = {},
): Promise<boolean> {
  if (opts.signal?.aborted) return false;
  const n = normalizeWs(wsUrl);
  if (!n) return false;
  const timeoutMs = opts.timeoutMs ?? 2000;
  const ctrl = new AbortController();
  const timer = setTimeout(() => ctrl.abort(), timeoutMs);
  const onAbort = () => ctrl.abort();
  opts.signal?.addEventListener("abort", onAbort);
  const href =
    typeof window !== "undefined" && isCloudFallbackHost(n)
      ? `/api/uplink/health?ws=${encodeURIComponent(n)}`
      : healthUrl(n);
  try {
    const r = await fetch(href, {
      method: "GET",
      cache: "no-store",
      signal: ctrl.signal,
    });
    if (!r.ok) return false;
    const body = (await r.json().catch(() => null)) as { ok?: boolean } | null;
    return body?.ok === true || r.ok;
  } catch {
    return false;
  } finally {
    clearTimeout(timer);
    opts.signal?.removeEventListener("abort", onAbort);
  }
}

/** Open WS with retries. Render Free often 404s HTTP while WS is still coming up. */
export function waitForSocket(
  url: string,
  opts: {
    signal: AbortSignal;
    onAttempt?: (n: number) => void;
    timeoutEachMs?: number;
    deadlineMs?: number;
    pauseMs?: number;
  },
): Promise<WebSocket | null> {
  const n = normalizeWs(url);
  if (!n) return Promise.resolve(null);
  const timeoutEachMs = opts.timeoutEachMs ?? 12_000;
  const deadline = Date.now() + (opts.deadlineMs ?? 120_000);
  const pauseMs = opts.pauseMs ?? 1500;

  return new Promise((resolve) => {
    let attempt = 0;
    let settled = false;
    let current: WebSocket | null = null;

    const finish = (ws: WebSocket | null) => {
      if (settled) return;
      settled = true;
      opts.signal.removeEventListener("abort", onAbort);
      resolve(ws);
    };

    const onAbort = () => {
      current?.close();
      finish(null);
    };
    opts.signal.addEventListener("abort", onAbort);

    const tryOnce = () => {
      if (settled || opts.signal.aborted || Date.now() >= deadline) {
        finish(null);
        return;
      }
      attempt += 1;
      opts.onAttempt?.(attempt);
      const ws = new WebSocket(n);
      current = ws;
      let opened = false;
      const timer = setTimeout(() => ws.close(), timeoutEachMs);
      ws.onopen = () => {
        opened = true;
        clearTimeout(timer);
        finish(ws);
      };
      ws.onerror = () => {
        /* onclose follows */
      };
      ws.onclose = () => {
        clearTimeout(timer);
        if (opened || settled || opts.signal.aborted) return;
        globalThis.setTimeout(tryOnce, pauseMs);
      };
    };

    tryOnce();
  });
}

/** Only two HUD uplinks: local jarvisd, then built-in Render. FFI is the Flutter in-process core. */
export async function resolveUplink(hooks: {
  signal: AbortSignal;
  onLog: (msg: string) => void;
  onStatus: (status: string) => void;
}): Promise<string | null> {
  if (hooks.signal.aborted) return null;

  hooks.onStatus("linking");
  const localOk = await pingHealth(LOCAL_WS, { signal: hooks.signal, timeoutMs: 1800 });
  if (hooks.signal.aborted) return null;
  if (localOk) {
    hooks.onLog("Uplink: local jarvisd");
    return LOCAL_WS;
  }

  const render = cloudWs();
  hooks.onStatus("waking");
  hooks.onLog("Local core down — uplink: Render");
  kickHealth(render);
  return render;
}
