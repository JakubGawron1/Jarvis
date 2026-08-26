"use client";

import dynamic from "next/dynamic";
import { useEffect, useRef, useState } from "react";
import HudFrame from "./components/HudFrame";
import type { VisualSpec } from "./components/VisualStage";
import {
  LOCAL_WS,
  isCloudFallbackHost,
  pingHealth,
  resolveUplink,
  statsUrl,
  uplinkLabel,
  waitForSocket,
  withToken,
} from "./lib/uplink";

const VisualStage = dynamic(() => import("./components/VisualStage"), { ssr: false });
const ArcReactor = dynamic(() => import("./components/ArcReactor"), { ssr: false });

type Msg = { role: "user" | "jarvis" | "sys"; text: string };

type Presence = {
  io_device?: string;
  leader?: string;
  devices?: { id: string; name: string; kind: string }[];
};

function isOverlay() {
  if (typeof window === "undefined") return false;
  return new URLSearchParams(window.location.search).get("overlay") === "1";
}

function pad(n: number) {
  return n.toString().padStart(2, "0");
}

function hudDeviceId(): string {
  try {
    const k = "jarvis_device_id";
    let id = sessionStorage.getItem(k);
    if (!id) {
      id = `hud-${crypto.randomUUID()}`;
      sessionStorage.setItem(k, id);
    }
    return id;
  } catch {
    return `hud-${Math.random().toString(36).slice(2)}`;
  }
}

function hudHello(deviceId: string) {
  const kind = /Win/.test(navigator.userAgent)
    ? "windows"
    : /Linux/.test(navigator.userAgent)
      ? "linux_desktop"
      : "windows";
  return withToken({
    type: "hello",
    device: {
      id: deviceId,
      name: "HUD",
      kind,
      boot: null,
      caps: {
        llm_local: false,
        llm_online: true,
        tts: true,
        stt: false,
        tools_os: false,
        rewrite_core: false,
        pull_core: true,
        mic: true,
        speaker: true,
      },
      core_version: "0.1.0",
      battery: null,
    },
  });
}

export default function Page() {
  const [overlay, setOverlay] = useState(false);
  const [input, setInput] = useState("");
  const [clock, setClock] = useState("--:--:--");
  const [log, setLog] = useState<Msg[]>([
    {
      role: "sys",
      text: "Interface online. Address me in Polish or English. Visual constructs on request.",
    },
  ]);
  const [stats, setStats] = useState({ cpu: 0, ram: "—", model: "—", version: "—" });
  const [presence, setPresence] = useState<Presence>({});
  const [status, setStatus] = useState("offline");
  const [activeUplink, setActiveUplink] = useState("");
  const [visual, setVisual] = useState<VisualSpec | null>(null);
  const [speaking, setSpeaking] = useState(false);
  const wsRef = useRef<WebSocket | null>(null);
  const abortRef = useRef<AbortController | null>(null);
  const genRef = useRef(0);
  const pendingRef = useRef<string[]>([]);
  const activeRef = useRef("");
  const deviceIdRef = useRef("");
  const autoLinkRef = useRef<() => Promise<void>>(async () => {});
  const logEnd = useRef<HTMLDivElement>(null);
  const audioRef = useRef<HTMLAudioElement | null>(null);

  function playSpeech(b64: string, mime = "audio/wav") {
    try {
      const bin = atob(b64);
      const bytes = new Uint8Array(bin.length);
      for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
      const url = URL.createObjectURL(new Blob([bytes], { type: mime }));
      audioRef.current?.pause();
      if (audioRef.current?.src) URL.revokeObjectURL(audioRef.current.src);
      const a = new Audio(url);
      audioRef.current = a;
      setSpeaking(true);
      const done = () => {
        setSpeaking(false);
        URL.revokeObjectURL(url);
      };
      a.onended = done;
      a.onerror = done;
      a.play().catch(done);
    } catch {
      setSpeaking(false);
    }
  }

  const bindSocket = (ws: WebSocket, gen: number, target: string, ac: AbortController) => {
    wsRef.current = ws;
    activeRef.current = target;
    setStatus("online");
    setActiveUplink(target);
    for (const frame of pendingRef.current) ws.send(frame);
    pendingRef.current = [];
    ws.send(JSON.stringify(hudHello(deviceIdRef.current || hudDeviceId())));
    ws.onmessage = (ev) => {
      if (gen !== genRef.current) return;
      try {
        const m = JSON.parse(ev.data);
        if (m.type === "reply") setLog((l) => [...l, { role: "jarvis", text: m.content }]);
        if (m.type === "confirm") setLog((l) => [...l, { role: "sys", text: "CONFIRM: " + m.prompt }]);
        if (m.type === "job_deferred") setLog((l) => [...l, { role: "sys", text: m.message }]);
        if (m.type === "error") {
          const hint =
            m.message === "unauthorized"
              ? "unauthorized — pairing token does not match Render"
              : m.message;
          setLog((l) => [...l, { role: "sys", text: hint }]);
        }
        if (m.type === "presence") setPresence(m);
        if (m.type === "core_waking") setLog((l) => [...l, { role: "sys", text: "Waking remote core…" }]);
        if (m.type === "visual") setVisual(m.spec as VisualSpec);
        if (m.type === "speech" && typeof m.audio_b64 === "string") {
          playSpeech(m.audio_b64, m.mime || "audio/mpeg");
        }
        if (m.type === "stats") {
          setStats({
            cpu: m.cpu,
            ram: `${Math.round(m.ram_used / 1e6)} / ${Math.round(m.ram_total / 1e6)} MB`,
            model: m.model,
            version: m.core_version,
          });
        }
      } catch {
        /* ignore */
      }
    };
    ws.onclose = () => {
      if (gen !== genRef.current || ac.signal.aborted) return;
      setStatus("offline");
      void autoLinkRef.current();
    };
  };

  const autoLink = async () => {
    const gen = ++genRef.current;
    abortRef.current?.abort();
    const ac = new AbortController();
    abortRef.current = ac;
    wsRef.current?.close();
    wsRef.current = null;
    setStatus("linking");

    const target = await resolveUplink({
      signal: ac.signal,
      onLog: (text) => {
        if (gen !== genRef.current) return;
        setLog((l) => [...l, { role: "sys", text }]);
      },
      onStatus: (s) => {
        if (gen !== genRef.current) return;
        setStatus(s);
      },
    });

    if (gen !== genRef.current || ac.signal.aborted) return;
    if (!target) {
      setStatus("offline");
      return;
    }

    const cloud = isCloudFallbackHost(target);
    const ws = await waitForSocket(target, {
      signal: ac.signal,
      timeoutEachMs: cloud ? 12_000 : 4000,
      deadlineMs: cloud ? 120_000 : 6000,
      onAttempt: (n) => {
        if (!cloud) return;
        if (n === 1 || n === 4 || n === 8) {
          setLog((l) => [
            ...l,
            {
              role: "sys",
              text: n === 1 ? "Opening WebSocket to Render…" : `Render WS retry ${n}…`,
            },
          ]);
        }
      },
    });

    if (gen !== genRef.current || ac.signal.aborted) return;
    if (!ws) {
      setStatus("offline");
      setLog((l) => [...l, { role: "sys", text: "No uplink (local jarvisd and Render both unreachable)." }]);
      return;
    }
    bindSocket(ws, gen, target, ac);
  };
  autoLinkRef.current = autoLink;

  useEffect(() => {
    deviceIdRef.current = hudDeviceId();
    void autoLink();
    const on = isOverlay();
    setOverlay(on);
    document.documentElement.classList.toggle("overlay", on);
    document.body.classList.toggle("overlay", on);
    const tick = () => {
      const d = new Date();
      setClock(`${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`);
    };
    tick();
    const c = setInterval(tick, 1000);
    const statsTimer = setInterval(async () => {
      const live = wsRef.current;
      if (!live || live.readyState !== WebSocket.OPEN) return;
      try {
        const r = await fetch(statsUrl(live.url || activeRef.current || LOCAL_WS));
        const m = await r.json();
        if (m.type === "stats" || m.cpu !== undefined) {
          setStats({
            cpu: m.cpu ?? 0,
            ram: `${Math.round((m.ram_used ?? 0) / 1e6)} / ${Math.round((m.ram_total ?? 0) / 1e6)} MB`,
            model: m.model ?? "—",
            version: m.core_version ?? "—",
          });
        }
      } catch {
        /* daemon down */
      }
    }, 3000);
    const failback = setInterval(async () => {
      if (wsRef.current?.readyState !== WebSocket.OPEN) return;
      if (!isCloudFallbackHost(activeRef.current)) return;
      const local = await pingHealth(LOCAL_WS, { timeoutMs: 1500 });
      if (local) {
        setLog((l) => [...l, { role: "sys", text: "Local jarvisd is back — switching uplink." }]);
        void autoLinkRef.current();
      }
    }, 8000);
    return () => {
      clearInterval(c);
      clearInterval(statsTimer);
      clearInterval(failback);
      abortRef.current?.abort();
      wsRef.current?.close();
      document.documentElement.classList.remove("overlay");
      document.body.classList.remove("overlay");
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    logEnd.current?.scrollIntoView({ behavior: "smooth" });
  }, [log]);

  const send = (content: string) => {
    if (!content.trim()) return;
    setLog((l) => [...l, { role: "user", text: content }]);
    const frame = JSON.stringify(
      withToken({
        type: "text",
        id: crypto.randomUUID(),
        content,
        device_id: deviceIdRef.current,
      }),
    );
    const live = wsRef.current;
    if (live && live.readyState === WebSocket.OPEN) live.send(frame);
    else {
      pendingRef.current.push(frame);
      setLog((l) => [...l, { role: "sys", text: "Queued — waiting for uplink." }]);
    }
    setInput("");
  };

  const dismissVisual = () => {
    setVisual(null);
    wsRef.current?.send(JSON.stringify(withToken({ type: "dismiss_visual" })));
  };

  const devices = presence.devices ?? [];
  const path = uplinkLabel(activeUplink);

  return (
    <div className="hud">
      <div className="hud-grid" />
      <div className="hud-scanlines" />
      <HudFrame />
      {visual && <VisualStage spec={visual} overlay={overlay} onDismiss={dismissVisual} />}
      <header className="hud-top">
        <span className="brand">J.A.R.V.I.S.</span>
        <span className="hud-clock">{clock}</span>
        <span className="hud-os">{overlay ? "JARVIS // OVERLAY" : "JARVIS · STARK OS"}</span>
      </header>
      <div className={"shell" + (overlay ? " overlay-shell" : "")}>
        <aside className="panel">
          <h1>Arc Reactor</h1>
          <ArcReactor status={status} cpu={stats.cpu} speaking={speaking} />
          <div className="stat">CPU <b>{stats.cpu.toFixed(0)}%</b></div>
          <div className="stat">RAM <b>{stats.ram}</b></div>
          <div className="stat">Model <b>{stats.model}</b></div>
          <div className="stat">Core <b>{stats.version}</b></div>
          <div className="stat">I/O <b>{presence.io_device ?? "—"}</b></div>
          <div className="stat">Leader <b>{presence.leader ?? "—"}</b></div>
          <div className="stat">Uplink <b>{path}</b></div>
          <h2>Mesh</h2>
          {devices.length === 0 && <div className="offline">No active nodes</div>}
          {devices.map((d) => (
            <div
              key={d.id}
              className={"device" + (d.id === presence.io_device ? " active" : "")}
              onClick={() =>
                wsRef.current?.send(
                  JSON.stringify(withToken({ type: "handoff_request", target_device: d.id })),
                )
              }
            >
              {d.name} · {d.kind}
            </div>
          ))}
        </aside>
        <main className="panel chat">
          <h1>Conversation</h1>
          <div className="log">
            {log.map((m, i) => (
              <div key={i} className={"msg " + m.role}>
                <b>{m.role}</b>
                {m.text}
              </div>
            ))}
            <div ref={logEnd} />
          </div>
          <form
            className="row"
            onSubmit={(e) => {
              e.preventDefault();
              send(input);
            }}
          >
            <input
              value={input}
              onChange={(e) => setInput(e.target.value)}
              placeholder="Awaiting input — PL / EN"
              aria-label="Command"
            />
            <button type="submit">Send</button>
            <button
              type="button"
              className="ptt"
              onClick={() => send(input || "Jarvis, status")}
              title="Push-to-talk"
            >
              MIC
            </button>
          </form>
        </main>
      </div>
    </div>
  );
}
