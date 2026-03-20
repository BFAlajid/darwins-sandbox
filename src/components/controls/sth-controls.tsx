'use client';

import { useSthStore } from '@/stores/sth-store';

interface Props {
  onPlay: () => void;
  onPause: () => void;
  onStep: () => void;
  onReset: (seed?: number) => void;
  onSetSpeed: (speed: number) => void;
  onToggleComparison: () => void;
}

const SPEED_PRESETS = [1, 2, 5, 10, 50];

export function SthControls({ onPlay, onPause, onStep, onReset, onSetSpeed, onToggleComparison }: Props) {
  const state = useSthStore((s) => s.state);
  const speed = useSthStore((s) => s.speed);
  const stats = useSthStore((s) => s.stats);
  const currentDay = useSthStore((s) => s.currentDay);
  const currentMonth = useSthStore((s) => s.currentMonth);
  const comparisonMode = useSthStore((s) => s.comparisonMode);

  const isRunning = state === 'running';
  const isLoading = state === 'loading';

  const tick = useSthStore((s) => s.currentTick);
  const hour = tick % 24;
  const timeLabel = `${hour.toString().padStart(2, '0')}:00`;

  return (
    <div className="flex items-center gap-2 md:gap-3 px-3 md:px-4 py-2 bg-gray-800 border-b border-gray-700 overflow-x-auto">
      {/* Play/Pause */}
      <button
        onClick={isRunning ? onPause : onPlay}
        disabled={isLoading}
        className="px-3 py-2 md:py-1.5 rounded bg-emerald-600 text-white font-medium text-sm hover:bg-emerald-500 disabled:opacity-40 min-w-[64px] md:min-w-[72px] active:bg-emerald-700"
      >
        {isLoading ? '...' : isRunning ? 'Pause' : 'Play'}
      </button>

      {/* Step */}
      <button
        onClick={onStep}
        disabled={isRunning || isLoading}
        className="px-3 py-2 md:py-1.5 rounded bg-gray-700 text-gray-300 text-sm hover:bg-gray-600 disabled:opacity-40 active:bg-gray-600"
      >
        Step
      </button>

      {/* Speed */}
      <div className="flex items-center gap-1 md:gap-1.5 ml-1 md:ml-2">
        <span className="text-gray-500 text-xs hidden md:inline">Speed:</span>
        {SPEED_PRESETS.map((s) => (
          <button
            key={s}
            onClick={() => onSetSpeed(s)}
            className={`px-2 py-1.5 md:py-1 rounded text-xs min-w-[32px] active:opacity-80 ${
              speed === s
                ? 'bg-emerald-600 text-white font-bold'
                : 'bg-gray-700 text-gray-400 hover:text-gray-200'
            }`}
          >
            {s}x
          </button>
        ))}
      </div>

      {/* Separator */}
      <div className="w-px h-5 bg-gray-700 mx-0.5 md:mx-1 hidden md:block" />

      {/* Reset */}
      <button
        onClick={() => onReset()}
        className="px-3 py-2 md:py-1.5 rounded bg-gray-700 text-gray-300 text-sm hover:bg-red-900 hover:text-red-200 disabled:opacity-40 active:bg-red-800"
      >
        Reset
      </button>

      {/* Separator */}
      <div className="w-px h-5 bg-gray-700 mx-0.5 md:mx-1 hidden md:block" />

      {/* Comparison mode toggle */}
      <button
        onClick={onToggleComparison}
        disabled={isLoading}
        className={`px-3 py-2 md:py-1.5 rounded text-sm font-medium transition-colors disabled:opacity-40 active:opacity-80 whitespace-nowrap ${
          comparisonMode
            ? 'bg-amber-600 text-white hover:bg-amber-500'
            : 'bg-gray-700 text-gray-300 hover:bg-gray-600'
        }`}
        title="Compare Urban vs Rural barangays side by side"
      >
        {comparisonMode ? 'Exit Compare' : 'Compare U/R'}
      </button>

      {/* Time display + stats (right-aligned) */}
      <div className="ml-auto flex items-center gap-2 md:gap-4 text-xs text-gray-400 font-mono flex-shrink-0">
        <span>
          Day <span className="text-gray-200">{currentDay}</span>
          <span className="hidden md:inline">
            {' | '}Month <span className="text-gray-200">{currentMonth}</span>
          </span>
          {' '}
          <span className="text-gray-500">{timeLabel}</span>
        </span>
        {stats && (
          <span className="hidden md:inline">
            Agents: <span className="text-gray-200">{stats.agentCount}</span>
          </span>
        )}
      </div>
    </div>
  );
}
