"use client";

type Props = { status: "online" | "offline" | string; cpu: number };

function r3(n: number) {
  return Math.round(n * 1000) / 1000;
}

export default function ArcReactor({ status, cpu }: Props) {
  const on = status === "online";
  const sweep = Math.max(8, Math.min(100, cpu));
  const hex = Array.from({ length: 6 }, (_, i) => {
    const a = (Math.PI / 3) * i - Math.PI / 2;
    return `${r3(100 + Math.cos(a) * 34)},${r3(100 + Math.sin(a) * 34)}`;
  }).join(" ");
  return (
    <div className={"reactor" + (on ? " reactor-on" : "")} aria-label={`Arc Reactor ${status}`}>
      <svg className="reactor-svg" viewBox="0 0 200 200">
        <defs>
          <radialGradient id="coreGlow" cx="50%" cy="50%" r="50%">
            <stop offset="0%" stopColor="#e8fbff" />
            <stop offset="45%" stopColor="#4ee3ff" />
            <stop offset="100%" stopColor="#0a3a4a" />
          </radialGradient>
          <linearGradient id="ringGold" x1="0" y1="0" x2="1" y2="1">
            <stop offset="0%" stopColor="#ffe0b8" />
            <stop offset="50%" stopColor="#ff8c3a" />
            <stop offset="100%" stopColor="#ffb347" />
          </linearGradient>
          <filter id="reactorBlur">
            <feGaussianBlur stdDeviation="1.4" />
          </filter>
        </defs>
        <circle cx="100" cy="100" r="94" fill="none" stroke="rgba(255,154,60,0.18)" strokeWidth="0.6" />
        {Array.from({ length: 36 }, (_, i) => {
          const a = (i / 36) * Math.PI * 2;
          const inner = i % 6 === 0 ? 88 : 91;
          return (
            <line
              key={i}
              x1={r3(100 + Math.cos(a) * inner)}
              y1={r3(100 + Math.sin(a) * inner)}
              x2={r3(100 + Math.cos(a) * 94)}
              y2={r3(100 + Math.sin(a) * 94)}
              stroke={i % 6 === 0 ? "#ffb347" : "rgba(255,154,60,0.45)"}
              strokeWidth={i % 6 === 0 ? 1.2 : 0.6}
            />
          );
        })}
        <circle
          className="reactor-spin-slow"
          cx="100"
          cy="100"
          r="82"
          fill="none"
          stroke="#ff9a3c"
          strokeWidth="1.4"
          strokeDasharray="3 9"
          filter="url(#reactorBlur)"
        />
        <circle cx="100" cy="100" r="72" fill="none" stroke="url(#ringGold)" strokeWidth="7" opacity="0.85" />
        <circle cx="100" cy="100" r="66" fill="none" stroke="rgba(255,224,184,0.35)" strokeWidth="1" />
        <circle
          className="reactor-spin"
          cx="100"
          cy="100"
          r="58"
          fill="none"
          stroke="#ff6a1a"
          strokeWidth="2"
          strokeDasharray="16 7 3 7"
        />
        <polygon
          points={hex}
          fill="none"
          stroke="rgba(78,227,255,0.55)"
          strokeWidth="1.2"
          className="reactor-hex"
        />
        <circle
          cx="100"
          cy="100"
          r="46"
          fill="none"
          stroke="rgba(78,227,255,0.4)"
          strokeWidth="9"
          strokeDasharray={`${sweep * 2.89} 289`}
          strokeLinecap="round"
          transform="rotate(-90 100 100)"
        />
        <circle cx="100" cy="100" r="28" fill="url(#coreGlow)" className="reactor-core" />
        <circle cx="100" cy="100" r="18" fill="none" stroke="rgba(232,251,255,0.55)" strokeWidth="1" />
        <circle cx="100" cy="100" r="14" fill="#041018" />
        <circle cx="100" cy="100" r="6" fill="#4ee3ff" />
        <path className="reactor-sweep" d="M100 100 L100 18 A82 82 0 0 1 142 28 Z" fill="rgba(78,227,255,0.12)" />
      </svg>
      <span className="reactor-label">{on ? "ONLINE" : "STANDBY"}</span>
    </div>
  );
}
