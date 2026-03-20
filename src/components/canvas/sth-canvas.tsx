'use client';

import { useEffect, useRef, useCallback, useState } from 'react';
import { renderSthFrame } from './sth-renderer';
import { renderMinimap } from './minimap';
import { useSthStore } from '@/stores/sth-store';
import { FLOATS_PER_AGENT, FacilityType } from '@/lib/sth-types';

const TRAIL_LENGTH = 10;     // number of snapshots to keep
const TRAIL_SAMPLE_RATE = 3; // sample every N frames (avoids per-frame overhead)

interface Camera {
  x: number;
  y: number;
  zoom: number;
}

interface Props {
  agentBufferRef: React.RefObject<Float32Array | null>;
  envBufferRef: React.RefObject<Float32Array | null>;
  facilityBufferRef: React.RefObject<Float32Array | null>;
  // Comparison mode: second simulation buffers
  agentBufferRef2: React.RefObject<Float32Array | null>;
  envBufferRef2: React.RefObject<Float32Array | null>;
  facilityBufferRef2: React.RefObject<Float32Array | null>;
  onBuildFacility: (type: FacilityType, x: number, y: number) => void;
  onSelectAgent: (index: number | null) => void;
}

export function SthCanvas({
  agentBufferRef,
  envBufferRef,
  facilityBufferRef,
  agentBufferRef2,
  envBufferRef2,
  facilityBufferRef2,
  onBuildFacility,
  onSelectAgent,
}: Props) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const ctxRef = useRef<CanvasRenderingContext2D | null>(null);
  const cameraRef = useRef<Camera>({ x: 0, y: 0, zoom: 1 });
  const camera2Ref = useRef<Camera>({ x: 0, y: 0, zoom: 1 });
  const rafRef = useRef<number>(0);
  const isDragging = useRef(false);
  const lastMouse = useRef({ x: 0, y: 0 });
  const hasAutoFit = useRef(false);
  const hasAutoFit2 = useRef(false);
  const [canvasSize, setCanvasSize] = useState({ width: 0, height: 0 });

  // Mouse position for facility placement preview
  const mousePosRef = useRef<{ x: number; y: number } | null>(null);
  // Track comparison mode changes to re-fit cameras
  const lastComparisonModeRef = useRef(false);
  // Touch state for pinch zoom
  const touchStateRef = useRef<{ dist: number; midX: number; midY: number } | null>(null);
  // Agent trail history ring buffers (oldest first, max TRAIL_LENGTH snapshots)
  const trailHistoryRef = useRef<Float32Array[]>([]);
  const trailHistory2Ref = useRef<Float32Array[]>([]);
  const trailFrameCounter = useRef(0);

  // Setup canvas and context
  const setupCanvas = useCallback(() => {
    const canvas = canvasRef.current;
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
      ctxRef.current = ctx;
    }

    setCanvasSize({ width: rect.width, height: rect.height });
  }, []);

  // Init + resize observer
  useEffect(() => {
    setupCanvas();

    const observer = new ResizeObserver(() => setupCanvas());
    if (canvasRef.current) {
      observer.observe(canvasRef.current.parentElement!);
    }

    return () => observer.disconnect();
  }, [setupCanvas]);

  // Mouse wheel zoom
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const onWheel = (e: WheelEvent) => {
      e.preventDefault();
      const comparisonMode = useSthStore.getState().comparisonMode;
      const rect = canvas.getBoundingClientRect();
      const mouseX = e.clientX - rect.left;

      const camera = comparisonMode && mouseX > canvasSize.width / 2
        ? camera2Ref.current
        : cameraRef.current;

      const adjustedMouseX = comparisonMode && mouseX > canvasSize.width / 2
        ? mouseX - canvasSize.width / 2
        : mouseX;
      const mouseY = e.clientY - rect.top;

      const zoomFactor = e.deltaY > 0 ? 0.9 : 1.1;
      const newZoom = Math.max(0.1, Math.min(10, camera.zoom * zoomFactor));

      const worldX = camera.x + adjustedMouseX / camera.zoom;
      const worldY = camera.y + mouseY / camera.zoom;

      camera.zoom = newZoom;
      camera.x = worldX - adjustedMouseX / newZoom;
      camera.y = worldY - mouseY / newZoom;
    };

    canvas.addEventListener('wheel', onWheel, { passive: false });
    return () => canvas.removeEventListener('wheel', onWheel);
  }, [canvasSize.width]);

  // Mouse drag pan
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    let dragCamera: Camera | null = null;

    const onMouseDown = (e: MouseEvent) => {
      const placementMode = useSthStore.getState().facilityPlacementMode;
      if (placementMode !== null) return;

      const comparisonMode = useSthStore.getState().comparisonMode;
      const rect = canvas.getBoundingClientRect();
      const mouseX = e.clientX - rect.left;

      if (comparisonMode && mouseX > canvasSize.width / 2) {
        dragCamera = camera2Ref.current;
      } else {
        dragCamera = cameraRef.current;
      }

      isDragging.current = true;
      lastMouse.current = { x: e.clientX, y: e.clientY };
    };

    const onMouseMove = (e: MouseEvent) => {
      const rect = canvas.getBoundingClientRect();
      const mouseX = e.clientX - rect.left;
      const compMode = useSthStore.getState().comparisonMode;
      const isRight = compMode && mouseX > canvasSize.width / 2;
      const cam = isRight ? camera2Ref.current : cameraRef.current;
      const adjX = isRight ? mouseX - canvasSize.width / 2 : mouseX;
      mousePosRef.current = {
        x: adjX / cam.zoom + cam.x,
        y: (e.clientY - rect.top) / cam.zoom + cam.y,
      };

      if (!isDragging.current || !dragCamera) return;
      const dx = e.clientX - lastMouse.current.x;
      const dy = e.clientY - lastMouse.current.y;
      dragCamera.x -= dx / dragCamera.zoom;
      dragCamera.y -= dy / dragCamera.zoom;
      lastMouse.current = { x: e.clientX, y: e.clientY };
    };

    const onMouseUp = () => {
      isDragging.current = false;
      dragCamera = null;
    };

    canvas.addEventListener('mousedown', onMouseDown);
    window.addEventListener('mousemove', onMouseMove);
    window.addEventListener('mouseup', onMouseUp);

    return () => {
      canvas.removeEventListener('mousedown', onMouseDown);
      window.removeEventListener('mousemove', onMouseMove);
      window.removeEventListener('mouseup', onMouseUp);
    };
  }, [canvasSize.width]);

  // Touch events for mobile: pan (1 finger) and pinch-zoom (2 fingers)
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const getTouchDist = (t1: Touch, t2: Touch) =>
      Math.hypot(t1.clientX - t2.clientX, t1.clientY - t2.clientY);

    const onTouchStart = (e: TouchEvent) => {
      e.preventDefault();
      if (e.touches.length === 1) {
        isDragging.current = true;
        lastMouse.current = { x: e.touches[0].clientX, y: e.touches[0].clientY };
      } else if (e.touches.length === 2) {
        isDragging.current = false;
        touchStateRef.current = {
          dist: getTouchDist(e.touches[0], e.touches[1]),
          midX: (e.touches[0].clientX + e.touches[1].clientX) / 2,
          midY: (e.touches[0].clientY + e.touches[1].clientY) / 2,
        };
      }
    };

    const onTouchMove = (e: TouchEvent) => {
      e.preventDefault();
      const camera = cameraRef.current;

      if (e.touches.length === 1 && isDragging.current) {
        const dx = e.touches[0].clientX - lastMouse.current.x;
        const dy = e.touches[0].clientY - lastMouse.current.y;
        camera.x -= dx / camera.zoom;
        camera.y -= dy / camera.zoom;
        lastMouse.current = { x: e.touches[0].clientX, y: e.touches[0].clientY };
      } else if (e.touches.length === 2 && touchStateRef.current) {
        const newDist = getTouchDist(e.touches[0], e.touches[1]);
        const scale = newDist / touchStateRef.current.dist;

        const rect = canvas.getBoundingClientRect();
        const midX = (e.touches[0].clientX + e.touches[1].clientX) / 2 - rect.left;
        const midY = (e.touches[0].clientY + e.touches[1].clientY) / 2 - rect.top;

        const worldX = camera.x + midX / camera.zoom;
        const worldY = camera.y + midY / camera.zoom;

        camera.zoom = Math.max(0.1, Math.min(10, camera.zoom * scale));
        camera.x = worldX - midX / camera.zoom;
        camera.y = worldY - midY / camera.zoom;

        touchStateRef.current.dist = newDist;
      }
    };

    const onTouchEnd = (e: TouchEvent) => {
      if (e.touches.length < 2) touchStateRef.current = null;
      if (e.touches.length === 0) isDragging.current = false;
    };

    canvas.addEventListener('touchstart', onTouchStart, { passive: false });
    canvas.addEventListener('touchmove', onTouchMove, { passive: false });
    canvas.addEventListener('touchend', onTouchEnd);

    return () => {
      canvas.removeEventListener('touchstart', onTouchStart);
      canvas.removeEventListener('touchmove', onTouchMove);
      canvas.removeEventListener('touchend', onTouchEnd);
    };
  }, []);

  // Convert mouse event to world coordinates
  const mouseToWorld = useCallback((e: React.MouseEvent<HTMLCanvasElement>) => {
    const canvas = canvasRef.current;
    if (!canvas) return { x: 0, y: 0 };
    const rect = canvas.getBoundingClientRect();
    const mouseX = e.clientX - rect.left;
    const comparisonMode = useSthStore.getState().comparisonMode;

    const isRightHalf = comparisonMode && mouseX > canvasSize.width / 2;
    const camera = isRightHalf ? camera2Ref.current : cameraRef.current;
    const adjustedX = isRightHalf ? mouseX - canvasSize.width / 2 : mouseX;

    return {
      x: adjustedX / camera.zoom + camera.x,
      y: (e.clientY - rect.top) / camera.zoom + camera.y,
    };
  }, [canvasSize.width]);

  // Click handler: facility placement or agent selection
  const handleClick = useCallback((e: React.MouseEvent<HTMLCanvasElement>) => {
    const store = useSthStore.getState();
    const pos = mouseToWorld(e);

    if (store.facilityPlacementMode !== null) {
      onBuildFacility(store.facilityPlacementMode, pos.x, pos.y);
      return;
    }

    const data = agentBufferRef.current;
    if (!data) return;

    const stats = store.stats;
    const count = stats?.agentCount ?? data.length / FLOATS_PER_AGENT;
    let bestIdx = -1;
    let bestDist = 400;

    for (let i = 0; i < count; i++) {
      const offset = i * FLOATS_PER_AGENT;
      if (offset + FLOATS_PER_AGENT > data.length) break;
      const dx = data[offset] - pos.x;
      const dy = data[offset + 1] - pos.y;
      const distSq = dx * dx + dy * dy;
      if (distSq < bestDist) {
        bestDist = distSq;
        bestIdx = i;
      }
    }

    onSelectAgent(bestIdx >= 0 ? bestIdx : null);
  }, [mouseToWorld, agentBufferRef, onBuildFacility, onSelectAgent]);

  // Main render loop
  useEffect(() => {
    const loop = () => {
      const ctx = ctxRef.current;
      if (!ctx) {
        rafRef.current = requestAnimationFrame(loop);
        return;
      }

      const store = useSthStore.getState();
      const {
        worldWidth,
        worldHeight,
        envGridWidth,
        envGridHeight,
        envCellSize,
        showContaminationHeatmap,
        showFacilities,
        showMinimap,
        showAgentPaths,
        dataLens,
        selectedAgent,
        facilityPlacementMode,
        currentTick,
        comparisonMode,
        envGridWidth2,
        envGridHeight2,
        envCellSize2,
        stats2,
      } = store;

      const agentCount = store.stats?.agentCount ?? 0;

      // Snapshot agent positions for trail rendering
      if (showAgentPaths && agentCount > 0) {
        trailFrameCounter.current++;
        if (trailFrameCounter.current >= TRAIL_SAMPLE_RATE) {
          trailFrameCounter.current = 0;

          // Sim1 trails
          if (agentBufferRef.current) {
            const buf = agentBufferRef.current;
            const snap = new Float32Array(agentCount * 2);
            for (let i = 0; i < agentCount; i++) {
              const off = i * FLOATS_PER_AGENT;
              if (off + 1 < buf.length) {
                snap[i * 2] = buf[off];
                snap[i * 2 + 1] = buf[off + 1];
              }
            }
            const h1 = trailHistoryRef.current;
            h1.push(snap);
            if (h1.length > TRAIL_LENGTH) h1.shift();
          }

          // Sim2 trails (comparison mode)
          const ac2 = stats2?.agentCount ?? 0;
          if (comparisonMode && agentBufferRef2.current && ac2 > 0) {
            const buf2 = agentBufferRef2.current;
            const snap2 = new Float32Array(ac2 * 2);
            for (let i = 0; i < ac2; i++) {
              const off = i * FLOATS_PER_AGENT;
              if (off + 1 < buf2.length) {
                snap2[i * 2] = buf2[off];
                snap2[i * 2 + 1] = buf2[off + 1];
              }
            }
            const h2 = trailHistory2Ref.current;
            h2.push(snap2);
            if (h2.length > TRAIL_LENGTH) h2.shift();
          }
        }
      } else if (!showAgentPaths && trailHistoryRef.current.length > 0) {
        trailHistoryRef.current = [];
        trailHistory2Ref.current = [];
      }

      // Reset auto-fit when comparison mode changes
      if (comparisonMode !== lastComparisonModeRef.current) {
        lastComparisonModeRef.current = comparisonMode;
        hasAutoFit.current = false;
        hasAutoFit2.current = false;
      }

      if (comparisonMode) {
        const halfWidth = canvasSize.width / 2;
        const dpr = window.devicePixelRatio || 1;

        if (!hasAutoFit.current && worldWidth > 0 && worldHeight > 0 && halfWidth > 0) {
          const padding = 20;
          const zoom = Math.min(
            (halfWidth - padding * 2) / worldWidth,
            (canvasSize.height - padding * 2) / worldHeight,
          );
          const cam1 = cameraRef.current;
          cam1.zoom = zoom;
          cam1.x = -(halfWidth / zoom - worldWidth) / 2;
          cam1.y = -(canvasSize.height / zoom - worldHeight) / 2;

          const cam2 = camera2Ref.current;
          cam2.zoom = zoom;
          cam2.x = -(halfWidth / zoom - worldWidth) / 2;
          cam2.y = -(canvasSize.height / zoom - worldHeight) / 2;

          hasAutoFit.current = true;
          hasAutoFit2.current = true;
        }

        renderSthFrame(ctx, agentBufferRef.current, envBufferRef.current, facilityBufferRef.current,
          agentCount, envGridWidth, envGridHeight, envCellSize, cameraRef.current,
          halfWidth, canvasSize.height, worldWidth, worldHeight,
          showContaminationHeatmap, showFacilities, dataLens, selectedAgent, currentTick, 0,
          showAgentPaths, trailHistoryRef.current);

        const agentCount2 = stats2?.agentCount ?? 0;
        renderSthFrame(ctx, agentBufferRef2.current, envBufferRef2.current, facilityBufferRef2.current,
          agentCount2, envGridWidth2, envGridHeight2, envCellSize2, camera2Ref.current,
          halfWidth, canvasSize.height, worldWidth, worldHeight,
          showContaminationHeatmap, showFacilities, dataLens, null, currentTick, halfWidth,
          showAgentPaths, trailHistory2Ref.current);

        // Draw divider line
        ctx.resetTransform();
        ctx.scale(dpr, dpr);
        ctx.strokeStyle = '#4B5563';
        ctx.lineWidth = 2;
        ctx.beginPath();
        ctx.moveTo(halfWidth, 0);
        ctx.lineTo(halfWidth, canvasSize.height);
        ctx.stroke();

        // Draw labels
        ctx.font = 'bold 14px sans-serif';
        ctx.textAlign = 'center';
        ctx.textBaseline = 'top';

        ctx.fillStyle = 'rgba(0, 0, 0, 0.6)';
        ctx.fillRect(halfWidth / 2 - 80, 8, 160, 28);
        ctx.fillStyle = '#F59E0B';
        ctx.fillText('Urban: Guadalupe', halfWidth / 2, 14);

        const urbanPrev = store.stats?.prevalenceAny ?? 0;
        ctx.font = 'bold 11px monospace';
        ctx.fillStyle = prevalenceBadgeColor(urbanPrev);
        ctx.fillText(`STH: ${(urbanPrev * 100).toFixed(1)}%`, halfWidth / 2, 30);

        ctx.font = 'bold 14px sans-serif';
        ctx.fillStyle = 'rgba(0, 0, 0, 0.6)';
        ctx.fillRect(halfWidth + halfWidth / 2 - 80, 8, 160, 28);
        ctx.fillStyle = '#34D399';
        ctx.fillText('Rural: Sudlon II', halfWidth + halfWidth / 2, 14);

        const ruralPrev = stats2?.prevalenceAny ?? 0;
        ctx.font = 'bold 11px monospace';
        ctx.fillStyle = prevalenceBadgeColor(ruralPrev);
        ctx.fillText(`STH: ${(ruralPrev * 100).toFixed(1)}%`, halfWidth + halfWidth / 2, 30);

      } else {
        // --- SINGLE MODE ---
        if (!hasAutoFit.current && worldWidth > 0 && worldHeight > 0 && canvasSize.width > 0) {
          const padding = 20;
          const zoom = Math.min(
            (canvasSize.width - padding * 2) / worldWidth,
            (canvasSize.height - padding * 2) / worldHeight,
          );
          const camera = cameraRef.current;
          camera.zoom = zoom;
          camera.x = -(canvasSize.width / zoom - worldWidth) / 2;
          camera.y = -(canvasSize.height / zoom - worldHeight) / 2;
          hasAutoFit.current = true;
        }

        renderSthFrame(ctx, agentBufferRef.current, envBufferRef.current, facilityBufferRef.current,
          agentCount, envGridWidth, envGridHeight, envCellSize, cameraRef.current,
          canvasSize.width, canvasSize.height, worldWidth, worldHeight,
          showContaminationHeatmap, showFacilities, dataLens, selectedAgent, currentTick, 0,
          showAgentPaths, trailHistoryRef.current);

        // Facility placement preview ghost
        if (facilityPlacementMode !== null && mousePosRef.current) {
          drawPlacementPreview(ctx, facilityPlacementMode, mousePosRef.current, cameraRef.current.zoom);
        }

        // Minimap
        if (showMinimap) {
          renderMinimap(
            ctx, agentBufferRef.current, facilityBufferRef.current, agentCount,
            cameraRef.current, canvasSize.width, canvasSize.height, worldWidth, worldHeight,
          );
        }
      }

      rafRef.current = requestAnimationFrame(loop);
    };

    rafRef.current = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(rafRef.current);
  }, [canvasSize, agentBufferRef, envBufferRef, facilityBufferRef, agentBufferRef2, envBufferRef2, facilityBufferRef2]);

  const cursorClass = useSthStore((s) =>
    s.facilityPlacementMode !== null ? 'cursor-cell' : 'cursor-crosshair',
  );

  return (
    <div className="relative w-full h-full touch-none">
      <canvas
        ref={canvasRef}
        className={`absolute inset-0 w-full h-full ${cursorClass}`}
        style={{ display: 'block' }}
        onClick={handleClick}
        role="img"
        aria-label="STH simulation visualization showing agent positions, infection status, and environmental contamination"
      />
    </div>
  );
}

/** Draw a ghost preview of the facility at the mouse position */
function drawPlacementPreview(
  ctx: CanvasRenderingContext2D,
  facilityType: FacilityType,
  pos: { x: number; y: number },
  zoom: number,
): void {
  ctx.save();
  ctx.globalAlpha = 0.5;
  ctx.font = `bold ${Math.max(8, 12 / zoom)}px monospace`;
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';

  switch (facilityType) {
    case FacilityType.Latrine: {
      ctx.fillStyle = '#8B5E3C';
      ctx.fillRect(pos.x - 5, pos.y - 5, 10, 10);
      ctx.fillStyle = '#fff';
      ctx.fillText('T', pos.x, pos.y + 1);
      break;
    }
    case FacilityType.WaterPump: {
      ctx.fillStyle = '#3B82F6';
      ctx.beginPath();
      ctx.arc(pos.x, pos.y, 6, 0, Math.PI * 2);
      ctx.fill();
      ctx.fillStyle = '#fff';
      ctx.fillText('W', pos.x, pos.y + 1);
      break;
    }
    case FacilityType.HandwashStation: {
      ctx.fillStyle = '#22C55E';
      ctx.beginPath();
      ctx.arc(pos.x, pos.y, 6, 0, Math.PI * 2);
      ctx.fill();
      ctx.fillStyle = '#fff';
      ctx.fillText('H', pos.x, pos.y + 1);
      break;
    }
  }

  ctx.strokeStyle = '#ffffff';
  ctx.lineWidth = 1 / zoom;
  ctx.setLineDash([4 / zoom, 4 / zoom]);
  ctx.beginPath();
  ctx.arc(pos.x, pos.y, 30, 0, Math.PI * 2);
  ctx.stroke();
  ctx.setLineDash([]);

  ctx.restore();
}

/** Return a CSS color for a prevalence value badge */
function prevalenceBadgeColor(value: number): string {
  if (value >= 0.5) return '#F87171';
  if (value >= 0.2) return '#FB923C';
  if (value >= 0.05) return '#FBBF24';
  return '#4ADE80';
}
