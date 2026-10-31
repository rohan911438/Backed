import { useEffect, useRef, useState } from "react";
import type { Reading } from "./near";

// Each dot is one NEP-141 token held by NEAR Intents, placed on a slowly
// turning sphere and colored by its real backing status.

const COLORS = { backed: "--ok", "UNDER-BACKED": "--warn", error: "--muted" } as const;

export default function DotField({ readings }: { readings: Reading[] }) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const [hover, setHover] = useState<{ x: number; y: number; r: Reading } | null>(null);

  useEffect(() => {
    const c = canvas.current;
    if (!c || readings.length === 0) return;
    const ctx = c.getContext("2d")!;
    const reduce = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    const n = readings.length;
    // Fibonacci sphere: even spacing for any number of dots.
    const pts = readings.map((r, i) => {
      const y = 1 - (2 * (i + 0.5)) / n;
      const rad = Math.sqrt(1 - y * y);
      const th = Math.PI * (3 - Math.sqrt(5)) * i;
      return { x: Math.cos(th) * rad, y, z: Math.sin(th) * rad, r };
    });
    let angle = 0.6;
    let frame = 0;
    let projected: { sx: number; sy: number; s: number; r: Reading }[] = [];

    const css = getComputedStyle(document.documentElement);
    const color = (k: keyof typeof COLORS) => css.getPropertyValue(COLORS[k]).trim();

    const draw = () => {
      const dpr = window.devicePixelRatio || 1;
      const w = c.clientWidth;
      const h = c.clientHeight;
      if (c.width !== w * dpr) {
        c.width = w * dpr;
        c.height = h * dpr;
      }
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      ctx.clearRect(0, 0, w, h);
      const R = Math.min(w, h) * 0.42;
      const cos = Math.cos(angle);
      const sin = Math.sin(angle);
      projected = pts
        .map((p) => {
          const x = p.x * cos - p.z * sin;
          const z = p.x * sin + p.z * cos;
          const tilt = 0.35;
          const y = p.y * Math.cos(tilt) - z * Math.sin(tilt);
          const z2 = p.y * Math.sin(tilt) + z * Math.cos(tilt);
          const s = (z2 + 2) / 3; // depth 0.33..1
          return { sx: w / 2 + x * R, sy: h / 2 + y * R, s, r: p.r, z: z2 };
        })
        .sort((a, b) => a.z - b.z);
      for (const p of projected) {
        const issue = p.r.status !== "backed";
        ctx.globalAlpha = issue ? 1 : 0.25 + 0.75 * p.s;
        ctx.fillStyle = color(p.r.status);
        ctx.beginPath();
        ctx.arc(p.sx, p.sy, (issue ? 4.2 : 2.2) * p.s + 0.6, 0, Math.PI * 2);
        ctx.fill();
        if (issue) {
          ctx.globalAlpha = 0.18;
          ctx.beginPath();
          ctx.arc(p.sx, p.sy, 12 * p.s, 0, Math.PI * 2);
          ctx.fill();
        }
      }
      ctx.globalAlpha = 1;
      if (!reduce) {
        angle += 0.0018;
        frame = requestAnimationFrame(draw);
      }
    };
    draw();

    const onMove = (e: PointerEvent) => {
      const rect = c.getBoundingClientRect();
      const mx = e.clientX - rect.left;
      const my = e.clientY - rect.top;
      let best: (typeof projected)[number] | null = null;
      let bestD = 14 * 14;
      for (const p of projected) {
        const d = (p.sx - mx) ** 2 + (p.sy - my) ** 2;
        if (d < bestD && p.s > 0.55) {
          bestD = d;
          best = p;
        }
      }
      setHover(best ? { x: best.sx, y: best.sy, r: best.r } : null);
    };
    const onLeave = () => setHover(null);
    c.addEventListener("pointermove", onMove);
    c.addEventListener("pointerleave", onLeave);
    return () => {
      cancelAnimationFrame(frame);
      c.removeEventListener("pointermove", onMove);
      c.removeEventListener("pointerleave", onLeave);
    };
  }, [readings]);

  return (
    <div className="dotfield">
      <canvas ref={canvas} aria-label="Sphere of NEAR Intents tokens colored by backing status" role="img" />
      {hover && (
        <div className="dot-tip" style={{ left: hover.x, top: hover.y }}>
          <strong>{hover.r.symbol ?? hover.r.token.slice(0, 18)}</strong>
          <span className={`pill pill-${hover.r.status === "backed" ? "ok" : hover.r.status === "error" ? "muted" : "warn"}`}>
            {hover.r.status === "backed" ? "backed" : hover.r.status === "error" ? "unreadable" : "under-backed"}
          </span>
        </div>
      )}
    </div>
  );
}
