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

  useEffect(() => {
    const loop = () => {
      const ctx = ctxRef.current;
      if (ctx) {
        const { worldWidth, worldHeight } = useSimulationStore.getState();
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
