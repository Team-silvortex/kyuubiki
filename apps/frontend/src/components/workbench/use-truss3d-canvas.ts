"use client";

import { useEffect, useRef } from "react";
import type { SceneBufferSet } from "./workbench-truss3d-webgl-scene";
import { createTruss3dWebglRenderer, type Truss3dDrawView } from "./workbench-truss3d-webgl-renderer";

export function useTruss3dCanvas(scene: SceneBufferSet, view: Truss3dDrawView) {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const latest = useRef({ scene, view });
  const invalidateRef = useRef<(() => void) | null>(null);

  useEffect(() => {
    latest.current = { scene, view };
    invalidateRef.current?.();
  }, [scene, view.projected3d, view.camera, view.projectionMode]);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const gl = canvas.getContext("webgl", { antialias: true, alpha: true });
    if (!gl) return;
    let renderer = createTruss3dWebglRenderer(gl);
    let frame: number | null = null;
    const paint = () => {
      frame = null;
      if (!renderer || gl.isContextLost()) return;
      const rect = canvas.getBoundingClientRect();
      const ratio = window.devicePixelRatio || 1;
      const width = Math.max(1, Math.round(rect.width * ratio));
      const height = Math.max(1, Math.round(rect.height * ratio));
      if (canvas.width !== width) canvas.width = width;
      if (canvas.height !== height) canvas.height = height;
      renderer.draw(latest.current.scene, latest.current.view, width, height);
    };
    const invalidate = () => {
      if (frame === null) frame = window.requestAnimationFrame(paint);
    };
    const cancel = () => {
      if (frame !== null) window.cancelAnimationFrame(frame);
      frame = null;
    };
    const lost = (event: Event) => {
      event.preventDefault();
      cancel();
      renderer?.dispose();
      renderer = null;
    };
    const restored = () => {
      renderer?.dispose();
      renderer = createTruss3dWebglRenderer(gl);
      invalidate();
    };
    invalidateRef.current = invalidate;
    const observer = typeof ResizeObserver === "undefined" ? null : new ResizeObserver(invalidate);
    observer?.observe(canvas);
    window.addEventListener("resize", invalidate);
    canvas.addEventListener("webglcontextlost", lost);
    canvas.addEventListener("webglcontextrestored", restored);
    invalidate();
    return () => {
      invalidateRef.current = null;
      cancel();
      observer?.disconnect();
      window.removeEventListener("resize", invalidate);
      canvas.removeEventListener("webglcontextlost", lost);
      canvas.removeEventListener("webglcontextrestored", restored);
      renderer?.dispose();
    };
  }, []);
  return canvasRef;
}
