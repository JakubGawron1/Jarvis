"use client";

import { useEffect, useRef, useState } from "react";
import * as THREE from "three";

export type VisualKind = "scene3d" | "slides" | "diagram" | "video";

export type VisualSpec = {
  kind: VisualKind;
  title: string;
  subtitle?: string | null;
  scene3d?: {
    camera_z?: number;
    bodies?: {
      id: string;
      shape?: string;
      radius?: number;
      color?: string;
      glow?: boolean;
      orbit?: { radius?: number; speed?: number; tilt?: number } | null;
      label?: string | null;
    }[];
    links?: [number, number][];
    particles?: number;
    neural?: boolean;
  } | null;
  slides?: { title: string; bullets?: string[] }[] | null;
  diagram?: { nodes: string[]; edges?: [number, number][] } | null;
  video?: { duration_sec?: number; caption?: string | null } | null;
};

type Props = {
  spec: VisualSpec;
  overlay?: boolean;
  onDismiss: () => void;
};

export default function VisualStage({ spec, overlay, onDismiss }: Props) {
  const host = useRef<HTMLDivElement>(null);
  const [slide, setSlide] = useState(0);
  const slides = spec.slides ?? [];

  useEffect(() => {
    setSlide(0);
  }, [spec.title, spec.kind]);

  useEffect(() => {
    const el = host.current;
    if (!el) return;

    const scene = new THREE.Scene();
    scene.fog = new THREE.FogExp2(0x050308, overlay ? 0.028 : 0.035);

    const camZ = spec.scene3d?.camera_z ?? 8;
    const camera = new THREE.PerspectiveCamera(48, 1, 0.1, 80);
    camera.position.set(0, 0.4, camZ);

    const renderer = new THREE.WebGLRenderer({
      antialias: true,
      alpha: true,
      powerPreference: "high-performance",
    });
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    renderer.setClearColor(0x000000, 0);
    el.appendChild(renderer.domElement);

    const ambient = new THREE.AmbientLight(0xffc078, 0.45);
    scene.add(ambient);
    const key = new THREE.PointLight(0xff8c3a, 40, 40);
    key.position.set(2, 3, 6);
    scene.add(key);
    const fill = new THREE.PointLight(0x4ee3ff, spec.scene3d?.neural ? 8 : 18, 30);
    fill.position.set(-4, -1, 3);
    scene.add(fill);

    const group = new THREE.Group();
    scene.add(group);

    const bodies = spec.scene3d?.bodies ?? [];
    const meshes: THREE.Mesh[] = [];
    const pivots: THREE.Group[] = [];

    bodies.forEach((b, i) => {
      const radius = b.radius ?? 0.2;
      const color = new THREE.Color(b.color || "#ff8c3a");
      const geo =
        b.shape === "box"
          ? new THREE.BoxGeometry(radius * 1.6, radius * 1.6, radius * 1.6)
          : new THREE.SphereGeometry(radius, 24, 18);
      const mat = new THREE.MeshStandardMaterial({
        color,
        emissive: color,
        emissiveIntensity: b.glow ? 1.4 : 0.35,
        roughness: 0.25,
        metalness: 0.55,
        transparent: true,
        opacity: 0.92,
      });
      const mesh = new THREE.Mesh(geo, mat);
      if (b.glow) {
        const halo = new THREE.Mesh(
          new THREE.SphereGeometry(radius * 1.85, 16, 12),
          new THREE.MeshBasicMaterial({
            color,
            transparent: true,
            opacity: 0.14,
            blending: THREE.AdditiveBlending,
            depthWrite: false,
          }),
        );
        mesh.add(halo);
      }
      const pivot = new THREE.Group();
      const orbit = b.orbit;
      if (orbit && Math.abs(orbit.tilt ?? 0) > 1.6) {
        mesh.position.set(orbit.radius ?? 1, orbit.tilt ?? 0, 0);
      } else if (!orbit) {
        mesh.position.set(0, 0, 0);
      }
      pivot.add(mesh);
      if (orbit) {
        pivot.rotation.x = Math.abs(orbit.tilt ?? 0) > 1.6 ? 0 : orbit.tilt ?? 0;
      }
      group.add(pivot);
      meshes.push(mesh);
      pivots.push(pivot);
    });

    const linkLines: THREE.Line[] = [];
    const links = spec.scene3d?.links ?? [];
    links.forEach(([a, b]) => {
      const geo = new THREE.BufferGeometry().setFromPoints([
        new THREE.Vector3(),
        new THREE.Vector3(),
      ]);
      const line = new THREE.Line(
        geo,
        new THREE.LineBasicMaterial({
          color: 0xff8c3a,
          transparent: true,
          opacity: 0.28,
          blending: THREE.AdditiveBlending,
        }),
      );
      group.add(line);
      linkLines.push(line);
    });

    const nParticles = spec.scene3d?.particles ?? (spec.scene3d?.neural ? 700 : 120);
    const pGeo = new THREE.BufferGeometry();
    const pPos = new Float32Array(nParticles * 3);
    for (let i = 0; i < nParticles; i++) {
      const r = 1.2 + Math.random() * 6.5;
      const th = Math.random() * Math.PI * 2;
      const ph = Math.acos(2 * Math.random() - 1);
      pPos[i * 3] = r * Math.sin(ph) * Math.cos(th);
      pPos[i * 3 + 1] = r * Math.cos(ph) * 0.7;
      pPos[i * 3 + 2] = r * Math.sin(ph) * Math.sin(th);
    }
    pGeo.setAttribute("position", new THREE.BufferAttribute(pPos, 3));
    const points = new THREE.Points(
      pGeo,
      new THREE.PointsMaterial({
        color: 0xffb347,
        size: 0.035,
        transparent: true,
        opacity: 0.7,
        blending: THREE.AdditiveBlending,
        depthWrite: false,
      }),
    );
    group.add(points);

    if (spec.scene3d?.neural || bodies.length === 0) {
      const ring = new THREE.Mesh(
        new THREE.TorusGeometry(3.2, 0.012, 8, 96),
        new THREE.MeshBasicMaterial({
          color: 0xff8c3a,
          transparent: true,
          opacity: 0.35,
        }),
      );
      ring.rotation.x = Math.PI / 2.4;
      group.add(ring);
    }

    const clock = new THREE.Clock();
    let raf = 0;
    const videoBoost = spec.kind === "video" ? 1.8 : 1;

    const resize = () => {
      const w = el.clientWidth || window.innerWidth;
      const h = el.clientHeight || window.innerHeight;
      camera.aspect = w / Math.max(h, 1);
      camera.updateProjectionMatrix();
      renderer.setSize(w, h, false);
    };
    resize();
    const ro = new ResizeObserver(resize);
    ro.observe(el);

    const tick = () => {
      const t = clock.getElapsedTime() * videoBoost;
      group.rotation.y = t * 0.08;
      points.rotation.y = t * 0.05;
      bodies.forEach((b, i) => {
        const orbit = b.orbit;
        const pivot = pivots[i];
        const mesh = meshes[i];
        if (!orbit || !pivot || !mesh) return;
        const speed = (orbit.speed ?? 1) * (0.7 + (i % 5) * 0.05);
        const r = orbit.radius ?? 1.5;
        const ang = t * speed + i * 0.4;
        const tilt = orbit.tilt ?? 0;
        if (Math.abs(tilt) > 1.6) {
          mesh.position.set(Math.cos(ang) * r, tilt, Math.sin(ang) * r);
        } else {
          mesh.position.set(Math.cos(ang) * r, Math.sin(ang * 0.35) * r * 0.12, Math.sin(ang) * r);
        }
      });
      links.forEach(([a, b], i) => {
        const line = linkLines[i];
        const ma = meshes[a];
        const mb = meshes[b];
        if (!line || !ma || !mb) return;
        const pa = new THREE.Vector3();
        const pb = new THREE.Vector3();
        ma.getWorldPosition(pa);
        mb.getWorldPosition(pb);
        line.geometry.setFromPoints([pa, pb]);
      });
      camera.position.x = Math.sin(t * 0.12) * 0.6;
      camera.lookAt(0, 0, 0);
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
        if (obj instanceof THREE.Mesh || obj instanceof THREE.Line || obj instanceof THREE.Points) {
          obj.geometry.dispose();
          const mat = obj.material;
          if (Array.isArray(mat)) mat.forEach((m) => m.dispose());
          else mat.dispose();
        }
      });
    };
  }, [spec, overlay]);

  return (
    <div className={"visual-stage" + (overlay ? " visual-stage-overlay" : "")}>
      <div className="visual-canvas" ref={host} />
      <div className="visual-chrome">
        <div className="visual-title">
          <span className="visual-kicker">{spec.kind.replace("3d", " 3D")}</span>
          <h2>{spec.title}</h2>
          {spec.subtitle && <p>{spec.subtitle}</p>}
          {spec.video?.caption && <p className="visual-caption">{spec.video.caption}</p>}
        </div>
        {spec.kind === "slides" && slides.length > 0 && (
          <div className="visual-slides">
            <h3>{slides[slide]?.title}</h3>
            <ul>
              {(slides[slide]?.bullets ?? []).map((b) => (
                <li key={b}>{b}</li>
              ))}
            </ul>
            <div className="visual-slide-nav">
              <button type="button" onClick={() => setSlide((s) => Math.max(0, s - 1))}>
                ←
              </button>
              <span>
                {slide + 1} / {slides.length}
              </span>
              <button
                type="button"
                onClick={() => setSlide((s) => Math.min(slides.length - 1, s + 1))}
              >
                →
              </button>
            </div>
          </div>
        )}
        {spec.kind === "diagram" && spec.diagram && (
          <svg className="visual-diagram" viewBox="0 0 400 220">
            {spec.diagram.nodes.map((n, i) => {
              const x = 50 + (i % 4) * 90;
              const y = 50 + Math.floor(i / 4) * 90;
              return (
                <g key={n + i}>
                  <rect x={x} y={y} width="80" height="36" rx="6" />
                  <text x={x + 40} y={y + 22} textAnchor="middle">
                    {n.slice(0, 12)}
                  </text>
                </g>
              );
            })}
            {(spec.diagram.edges ?? []).map(([a, b], i) => {
              const ax = 90 + (a % 4) * 90;
              const ay = 68 + Math.floor(a / 4) * 90;
              const bx = 90 + (b % 4) * 90;
              const by = 68 + Math.floor(b / 4) * 90;
              return <line key={i} x1={ax} y1={ay} x2={bx} y2={by} />;
            })}
          </svg>
        )}
        <button type="button" className="visual-dismiss" onClick={onDismiss}>
          Zamknij hologram
        </button>
      </div>
    </div>
  );
}
