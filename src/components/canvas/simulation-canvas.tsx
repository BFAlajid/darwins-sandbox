'use client';

import { useEffect, useRef } from 'react';
import { useCanvas } from '@/hooks/use-canvas';
import { renderFrame } from './renderer';
import { useSimulationStore } from '@/stores/simulation-store';

interface Props {
  creatureBufferRef: React.RefObject<Float32Array | null>;
  foodBufferRef: React.RefObject<Float32Array | null>;
}

export function SimulationCanvas({ creatureBufferRef, foodBufferRef }: Props) {
  const { canvasRef, ctxRef, cameraRef, canvasSize } = useCanvas();
  const rafRef = useRef<number>(0);
  const hasAutoFit = useRef(false);

  useEffect(() => {
    const loop = () => {
      const ctx = ctxRef.current;
      if (ctx) {
        const { worldWidth, worldHeight } = useSimulationStore.getState();

        // Auto-fit camera to world on first valid frame
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

        renderFrame(
          ctx,
          creatureBufferRef.current,
          foodBufferRef.current,
          cameraRef.current,
          canvasSize.width,
          canvasSize.height,
          worldWidth,
          worldHeight,
        );
      }
      rafRef.current = requestAnimationFrame(loop);
    };

    rafRef.current = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(rafRef.current);
  }, [ctxRef, cameraRef, canvasSize, creatureBufferRef, foodBufferRef]);

  return (
    <canvas
      ref={canvasRef}
      className="w-full h-full cursor-crosshair"
      style={{ display: 'block' }}
    />
  );
}
