"use client";

import { useEffect, useRef } from "react";
import * as THREE from "three";

type Props = { status: "online" | "offline" | string; cpu: number; speaking?: boolean };

function fibonacciSphere(count: number, radius: number) {
  const pos = new Float32Array(count * 3);
  const golden = Math.PI * (3 - Math.sqrt(5));
  for (let i = 0; i < count; i++) {
    const y = 1 - (i / Math.max(count - 1, 1)) * 2;
    const r = Math.sqrt(Math.max(0, 1 - y * y));
    const theta = golden * i;
    pos[i * 3] = Math.cos(theta) * r * radius;
    pos[i * 3 + 1] = y * radius;
    pos[i * 3 + 2] = Math.sin(theta) * r * radius;
  }
  return pos;
}

export default function ArcReactor({ status, cpu, speaking = false }: Props) {
  const host = useRef<HTMLDivElement>(null);
  const speakingRef = useRef(speaking);
  const statusRef = useRef(status);
  const cpuRef = useRef(cpu);
  speakingRef.current = speaking;
  statusRef.current = status;
  cpuRef.current = cpu;

  useEffect(() => {
    const el = host.current;
    if (!el) return;

    const scene = new THREE.Scene();
    const camera = new THREE.PerspectiveCamera(42, 1, 0.1, 40);
    camera.position.z = 6.2;

    const renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true });
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    renderer.setClearColor(0x000000, 0);
    el.appendChild(renderer.domElement);

    const root = new THREE.Group();
    scene.add(root);

    const makePoints = (n: number, radius: number, size: number, color: number, opacity: number) => {
      const geo = new THREE.BufferGeometry();
      geo.setAttribute("position", new THREE.BufferAttribute(fibonacciSphere(n, radius), 3));
      const mat = new THREE.PointsMaterial({
        color,
        size,
        transparent: true,
        opacity,
        depthWrite: false,
        blending: THREE.AdditiveBlending,
        sizeAttenuation: true,
      });
      const pts = new THREE.Points(geo, mat);
      root.add(pts);
      return pts;
    };

    const core = makePoints(900, 0.55, 0.045, 0xffe8c0, 0.95);
    const mid = makePoints(1400, 1.15, 0.028, 0xff9a3c, 0.7);
    const outer = makePoints(1800, 1.85, 0.02, 0xff6a1a, 0.45);

    const ringGeo = new THREE.TorusGeometry(1.55, 0.012, 8, 96);
    const ringMat = new THREE.MeshBasicMaterial({
      color: 0xffb347,
      transparent: true,
      opacity: 0.55,
    });
    const ringA = new THREE.Mesh(ringGeo, ringMat);
    ringA.rotation.x = Math.PI / 2.4;
    const ringB = new THREE.Mesh(ringGeo, ringMat.clone());
    ringB.rotation.y = Math.PI / 3;
    ringB.scale.setScalar(1.12);
    root.add(ringA, ringB);

    const glow = new THREE.Mesh(
      new THREE.SphereGeometry(0.42, 24, 24),
      new THREE.MeshBasicMaterial({
        color: 0xffc078,
        transparent: true,
        opacity: 0.35,
        blending: THREE.AdditiveBlending,
      }),
    );
    root.add(glow);

    const resize = () => {
      const w = el.clientWidth || 220;
      const h = el.clientHeight || 220;
      camera.aspect = w / h;
      camera.updateProjectionMatrix();
      renderer.setSize(w, h, false);
    };
    resize();
    const ro = new ResizeObserver(resize);
    ro.observe(el);

    let raf = 0;
    const t0 = performance.now();
    const tick = () => {
      const t = (performance.now() - t0) / 1000;
      const talk = speakingRef.current;
      const on = statusRef.current === "online" || talk;
      const pulse = talk ? 1 + Math.sin(t * 14) * 0.12 : 1 + Math.sin(t * 2.2) * 0.03;
      const spin = talk ? 1.8 : on ? 0.55 : 0.18;
      core.rotation.y = t * 0.35 * spin;
      mid.rotation.y = -t * 0.22 * spin;
      mid.rotation.x = Math.sin(t * 0.4) * 0.15;
      outer.rotation.y = t * 0.12 * spin;
      outer.rotation.z = t * 0.08;
      ringA.rotation.z = t * 0.6 * spin;
      ringB.rotation.x = t * 0.35 * spin;
      glow.scale.setScalar(pulse);
      (glow.material as THREE.MeshBasicMaterial).opacity = talk ? 0.55 : 0.28;
      root.rotation.x = 0.18 + Math.sin(t * 0.3) * 0.05;
      renderer.render(scene, camera);
      raf = requestAnimationFrame(tick);
    };
    tick();

    return () => {
      cancelAnimationFrame(raf);
      ro.disconnect();
      renderer.dispose();
      el.removeChild(renderer.domElement);
      scene.traverse((obj) => {
        if (obj instanceof THREE.Points || obj instanceof THREE.Mesh) {
          obj.geometry.dispose();
          const m = obj.material;
          if (Array.isArray(m)) m.forEach((x) => x.dispose());
          else m.dispose();
        }
      });
    };
  }, []);

  const on = status === "online";
  const waking = status === "waking" || status === "linking";
  const label = speaking ? "SPEAKING" : on ? "ONLINE" : waking ? "WAKING" : "STANDBY";

  return (
    <div
      className={
        "reactor" + (speaking ? " reactor-speak" : on ? " reactor-on" : waking ? " reactor-wake" : "")
      }
      aria-label={`Arc Reactor ${label}`}
    >
      <div ref={host} className="reactor-canvas" />
      <span className="reactor-label">{label}</span>
    </div>
  );
}
