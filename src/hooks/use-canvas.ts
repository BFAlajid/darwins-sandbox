'use client';

import { useCallback, useEffect, useRef, useState } from 'react';

interface Camera {
  x: number;
  y: number;
  zoom: number;
}

export function useCanvas() {
  // WebGL canvas (bottom layer)
  const glCanvasRef = useRef<HTMLCanvasElement>(null);
  // 2D overlay canvas (top layer)
  const overlayCanvasRef = useRef<HTMLCanvasElement>(null);
  const overlayCtxRef = useRef<CanvasRenderingContext2D | null>(null);
  const cameraRef = useRef<Camera>({ x: 0, y: 0, zoom: 1 });
  const isDragging = useRef(false);
  const lastMouse = useRef({ x: 0, y: 0 });
  const [canvasSize, setCanvasSize] = useState({ width: 0, height: 0 });

  const setupOverlay = useCallback(() => {
    const canvas = overlayCanvasRef.current;
    if (!canvas) return;

    const rect = canvas.getBoundingClientRect();
    const dpr = window.devicePixelRatio || 1;
    canvas.width = rect.width * dpr;
    canvas.height = rect.height * dpr;
    canvas.style.width = rect.width + 'px';
    canvas.style.height = rect.height + 'px';

    const ctx = canvas.getContext('2d', { desynchronized: true });
    if (ctx) {
      ctx.scale(dpr, dpr);
      overlayCtxRef.current = ctx;
    }

    setCanvasSize({ width: rect.width, height: rect.height });
  }, []);

  useEffect(() => {
    setupOverlay();

    const observer = new ResizeObserver(() => setupOverlay());
    if (overlayCanvasRef.current) {
      observer.observe(overlayCanvasRef.current.parentElement!);
    }

    return () => observer.disconnect();
  }, [setupOverlay]);

  // Mouse wheel zoom (on overlay canvas since it's on top)
  useEffect(() => {
    const canvas = overlayCanvasRef.current;
    if (!canvas) return;

    const onWheel = (e: WheelEvent) => {
      e.preventDefault();
      const camera = cameraRef.current;
      const zoomFactor = e.deltaY > 0 ? 0.9 : 1.1;
      const newZoom = Math.max(0.1, Math.min(10, camera.zoom * zoomFactor));

      const rect = canvas.getBoundingClientRect();
      const mouseX = e.clientX - rect.left;
      const mouseY = e.clientY - rect.top;
      const worldX = camera.x + mouseX / camera.zoom;
      const worldY = camera.y + mouseY / camera.zoom;

      camera.zoom = newZoom;
      camera.x = worldX - mouseX / newZoom;
      camera.y = worldY - mouseY / newZoom;
    };

    canvas.addEventListener('wheel', onWheel, { passive: false });
    return () => canvas.removeEventListener('wheel', onWheel);
  }, []);

  // Mouse drag pan
  useEffect(() => {
    const canvas = overlayCanvasRef.current;
    if (!canvas) return;

    const onMouseDown = (e: MouseEvent) => {
      isDragging.current = true;
      lastMouse.current = { x: e.clientX, y: e.clientY };
    };

    const onMouseMove = (e: MouseEvent) => {
      if (!isDragging.current) return;
      const camera = cameraRef.current;
      const dx = e.clientX - lastMouse.current.x;
      const dy = e.clientY - lastMouse.current.y;
      camera.x -= dx / camera.zoom;
      camera.y -= dy / camera.zoom;
      lastMouse.current = { x: e.clientX, y: e.clientY };
    };

    const onMouseUp = () => {
      isDragging.current = false;
    };

    canvas.addEventListener('mousedown', onMouseDown);
    window.addEventListener('mousemove', onMouseMove);
    window.addEventListener('mouseup', onMouseUp);

    return () => {
      canvas.removeEventListener('mousedown', onMouseDown);
      window.removeEventListener('mousemove', onMouseMove);
      window.removeEventListener('mouseup', onMouseUp);
    };
  }, []);

  return { glCanvasRef, overlayCanvasRef, overlayCtxRef, cameraRef, canvasSize, setupOverlay };
}
