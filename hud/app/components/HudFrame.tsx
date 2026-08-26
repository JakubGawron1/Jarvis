export default function HudFrame() {
  return (
    <div className="hud-frame" aria-hidden>
      <svg className="hud-corners" viewBox="0 0 100 100" preserveAspectRatio="none">
        <path d="M6 24 V6 H24" />
        <path d="M8 20 V8 H20" />
        <path d="M76 6 H94 V24" />
        <path d="M80 8 H92 V20" />
        <path d="M6 76 V94 H24" />
        <path d="M8 80 V92 H20" />
        <path d="M76 94 H94 V76" />
        <path d="M80 92 H92 V80" />
      </svg>
      <div className="hud-ticks hud-ticks-top" />
      <div className="hud-ticks hud-ticks-bottom" />
      <div className="hud-mid hud-mid-left" />
      <div className="hud-mid hud-mid-right" />
    </div>
  );
}
