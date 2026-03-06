'use client';

import { useSimulationStore } from '@/stores/simulation-store';

interface Props {
  onPlay: () => void;
  onPause: () => void;
  onStep: () => void;
  onReset: (seed?: number) => void;
  onSetSpeed: (speed: number) => void;
}

const SPEED_PRESETS = [1, 2, 3, 5, 10, 50];

export function SimulationControls({ onPlay, onPause, onStep, onReset, onSetSpeed }: Props) {
  const state = useSimulationStore((s) => s.state);
  const speed = useSimulationStore((s) => s.speed);
  const stats = useSimulationStore((s) => s.stats);

  const isRunning = state === 'running';
  const isLoading = state === 'loading';

  return (
    <div className="flex items-center gap-3 px-4 py-2 bg-surface-light border-b border-surface-lighter">
      {/* Play/Pause */}
      <button
        onClick={isRunning ? onPause : onPlay}
        disabled={isLoading}
        className="px-3 py-1.5 rounded bg-accent text-black font-medium text-sm hover:opacity-90 disabled:opacity-40 min-w-[72px]"
      >
        {isLoading ? '...' : isRunning ? 'Pause' : 'Play'}
      </button>

      {/* Step */}
      <button
        onClick={onStep}
        disabled={isRunning || isLoading}
        className="px-3 py-1.5 rounded bg-surface-lighter text-gray-300 text-sm hover:bg-surface disabled:opacity-40"
        title="Single step (or press .)"
      >
        Step
      </button>

      {/* Speed */}
      <div className="flex items-center gap-1.5 ml-2">
        <span className="text-gray-500 text-xs">Speed:</span>
        {SPEED_PRESETS.map((s) => (
          <button
            key={s}
            onClick={() => onSetSpeed(s)}
            className={`px-2 py-1 rounded text-xs ${
              speed === s
                ? 'bg-accent text-black font-bold'
                : 'bg-surface-lighter text-gray-400 hover:text-gray-200'
            }`}
          >
            {s}x
          </button>
        ))}
      </div>

      {/* Reset */}
      <button
        onClick={() => onReset()}
        className="px-3 py-1.5 rounded bg-surface-lighter text-gray-300 text-sm hover:bg-accent-red hover:text-white ml-2"
      >
        Reset
      </button>

      {/* Stats */}
      <div className="ml-auto flex items-center gap-4 text-xs text-gray-400 font-mono">
        {stats && (
          <>
            <span>Tick: <span className="text-gray-200">{stats.tick.toLocaleString()}</span></span>
            <span>Gen: <span className="text-gray-200">{stats.generationMax}</span></span>
            <span>Pop: <span className="text-gray-200">{stats.creatureCount}</span></span>
            <span>Food: <span className="text-gray-200">{stats.foodCount}</span></span>
          </>
        )}
      </div>
    </div>
  );
}
