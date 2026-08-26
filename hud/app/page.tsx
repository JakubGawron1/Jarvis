"use client";

import dynamic from "next/dynamic";
import { useEffect, useRef, useState } from "react";
import ArcReactor from "./components/ArcReactor";
import HudFrame from "./components/HudFrame";
import type { VisualSpec } from "./components/VisualStage";

const VisualStage = dynamic(() => import("./components/VisualStage"), { ssr: false });

type Msg = { role: "user" | "jarvis" | "sys"; text: string };

type Presence = {
  io_device?: string;
  leader?: string;
  devices?: { id: string; name: string; kind: string }[];
};

function defaultWs() {
  if (typeof window === "undefined") return "ws://127.0.0.1:7420/ws";
  const q = new URLSearchParams(window.location.search).get("ws");
  return q || localStorage.getItem("jarvis_ws") || "ws://127.0.0.1:7420/ws";
}

function isOverlay() {
  if (typeof window === "undefined") return false;
  return new URLSearchParams(window.location.search).get("overlay") === "1";
}

function pad(n: number) {
  return n.toString().padStart(2, "0");
}

export default function Page() {
  const [wsUrl, setWsUrl] = useState("ws://127.0.0.1:7420/ws");
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
  const [visual, setVisual] = useState<VisualSpec | null>(null);
  const wsRef = useRef<WebSocket | null>(null);
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
      a.onended = () => URL.revokeObjectURL(url);
      a.play().catch(() => URL.revokeObjectURL(url));
    } catch {
      /* ignore decode */
    }
  }

  useEffect(() => {
    setWsUrl(defaultWs());
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
    return () => {
      clearInterval(c);
      document.documentElement.classList.remove("overlay");
      document.body.classList.remove("overlay");
    };
  }, []);

  useEffect(() => {
    logEnd.current?.scrollIntoView({ behavior: "smooth" });
  }, [log]);

  const connect = () => {
    wsRef.current?.close();
    const ws = new WebSocket(wsUrl);
    wsRef.current = ws;
    ws.onopen = () => setStatus("online");
    ws.onclose = () => setStatus("offline");
    ws.onerror = () => setStatus("offline");
    ws.onmessage = (ev) => {
      try {
        const m = JSON.parse(ev.data);
        if (m.type === "reply") setLog((l) => [...l, { role: "jarvis", text: m.content }]);
        if (m.type === "confirm") setLog((l) => [...l, { role: "sys", text: "CONFIRM: " + m.prompt }]);
        if (m.type === "job_deferred") setLog((l) => [...l, { role: "sys", text: m.message }]);
        if (m.type === "error") setLog((l) => [...l, { role: "sys", text: m.message }]);
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
  };

  useEffect(() => {
    connect();
    const t = setInterval(async () => {
      try {
        const http = wsUrl.replace("ws", "http").replace(/\/ws$/, "");
        const r = await fetch(`${http}/stats`);
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
    return () => {
      clearInterval(t);
      wsRef.current?.close();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [wsUrl]);

  const send = (content: string) => {
    if (!content.trim()) return;
    setLog((l) => [...l, { role: "user", text: content }]);
    wsRef.current?.send(JSON.stringify({ type: "text", id: crypto.randomUUID(), content }));
    setInput("");
  };

  const dismissVisual = () => {
    setVisual(null);
    wsRef.current?.send(JSON.stringify({ type: "dismiss_visual" }));
  };

  const devices = presence.devices ?? [];

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
          <ArcReactor status={status} cpu={stats.cpu} />
          <div className="stat">CPU <b>{stats.cpu.toFixed(0)}%</b></div>
          <div className="stat">RAM <b>{stats.ram}</b></div>
          <div className="stat">Model <b>{stats.model}</b></div>
          <div className="stat">Core <b>{stats.version}</b></div>
          <div className="stat">I/O <b>{presence.io_device ?? "—"}</b></div>
          <div className="stat">Leader <b>{presence.leader ?? "—"}</b></div>
          <h2>Mesh</h2>
          {devices.length === 0 && <div className="offline">No active nodes</div>}
          {devices.map((d) => (
            <div
              key={d.id}
              className={"device" + (d.id === presence.leader ? " active" : "")}
              onClick={() =>
                wsRef.current?.send(JSON.stringify({ type: "handoff_request", target_device: d.id }))
              }
            >
              {d.name} · {d.kind}
            </div>
          ))}
          <h2>Uplink</h2>
          <input value={wsUrl} onChange={(e) => setWsUrl(e.target.value)} aria-label="WebSocket endpoint" />
          <button type="button" onClick={connect} style={{ marginTop: 8, width: "100%" }}>
            Link
          </button>
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
