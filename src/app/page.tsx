'use client';

import { useEffect, useCallback } from 'react';
import { SimulationCanvas } from '@/components/canvas/simulation-canvas';
import { SimulationControls } from '@/components/controls/simulation-controls';
import { useSimulation } from '@/hooks/use-simulation';
import { useSimulationStore } from '@/stores/simulation-store';

export default function Home() {
  const {
    creatureBufferRef,
    foodBufferRef,
    play,
    pause,
    step,
    reset,
    setSpeed,
  } = useSimulation();

  const state = useSimulationStore((s) => s.state);
  const error = useSimulationStore((s) => s.error);
  const stats = useSimulationStore((s) => s.stats);

  // Auto-start at 3x speed after WASM loads
  useEffect(() => {
    if (state === 'paused' && !stats) {
      // First load — auto-start
      const timer = setTimeout(() => play(), 100);
      return () => clearTimeout(timer);
    }
  }, [state, stats, play]);

  // Keyboard shortcuts
  const handleKeyDown = useCallback((e: KeyboardEvent) => {
    if (e.target instanceof HTMLInputElement) return;

    switch (e.key) {
      case ' ':
        e.preventDefault();
        state === 'running' ? pause() : play();
        break;
      case '.':
        if (state !== 'running') step();
        break;
      case '1': setSpeed(1); break;
      case '2': setSpeed(2); break;
      case '3': setSpeed(3); break;
      case '4': setSpeed(5); break;
      case '5': setSpeed(10); break;
    }
  }, [state, play, pause, step, setSpeed]);

  useEffect(() => {
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [handleKeyDown]);

  return (
    <div className="h-screen flex flex-col">
      {/* Top controls bar */}
      <SimulationControls
        onPlay={play}
        onPause={pause}
        onStep={step}
        onReset={reset}
        onSetSpeed={setSpeed}
      />

      {/* Main content */}
      <div className="flex-1 relative overflow-hidden">
        {/* Canvas */}
        <SimulationCanvas
          creatureBufferRef={creatureBufferRef}
          foodBufferRef={foodBufferRef}
        />

        {/* Error overlay */}
        {error && (
          <div className="absolute inset-0 flex items-center justify-center bg-black/70">
            <div className="bg-surface-light p-6 rounded-lg max-w-md text-center">
              <h2 className="text-accent-red text-lg font-bold mb-2">Simulation Error</h2>
              <p className="text-gray-300 text-sm mb-4">{error}</p>
              <button
                onClick={() => reset()}
                className="px-4 py-2 bg-accent rounded text-black font-medium"
              >
                Reset Simulation
              </button>
            </div>
          </div>
        )}

        {/* Loading state */}
        {state === 'loading' && (
          <div className="absolute inset-0 flex items-center justify-center bg-black/50">
            <p className="text-gray-400 text-sm">Loading WASM simulation...</p>
          </div>
        )}
      </div>

      {/* Bottom status bar */}
      {stats && (
        <div className="px-4 py-1.5 bg-surface-light border-t border-surface-lighter flex items-center gap-6 text-xs font-mono text-gray-500">
          <span>Births: {stats.totalBirths.toLocaleString()}</span>
          <span>Deaths: {stats.totalDeaths.toLocaleString()}</span>
          <span className={stats.energyDriftPct > 10 ? 'text-yellow-500' : ''}>
            Energy drift: {stats.energyDriftPct.toFixed(1)}%
          </span>
          {stats.nanDeaths > 0 && (
            <span className="text-red-500">NaN deaths: {stats.nanDeaths}</span>
          )}
        </div>
      )}
    </div>
  );
}
