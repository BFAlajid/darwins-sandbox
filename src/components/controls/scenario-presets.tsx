'use client';

import { useState, useCallback } from 'react';
import { SCENARIO_PRESETS, type ScenarioPreset } from '@/lib/thesis-data';
import { useToastStore } from '@/stores/toast-store';

interface Props {
  onReset: (seed?: number, config?: string) => void;
}

export function ScenarioPresets({ onReset }: Props) {
  const [expanded, setExpanded] = useState(false);
  const [activePreset, setActivePreset] = useState<string | null>(null);

  const addToast = useToastStore((s) => s.addToast);

  const handlePreset = useCallback(
    (preset: ScenarioPreset) => {
      if (!preset.config) return;

      // Validate config values
      for (const [key, val] of Object.entries(preset.config)) {
        if (typeof val === 'number' && !isFinite(val)) {
          console.warn(`Invalid config value for ${key}: ${val}`);
          return;
        }
      }

      const configStr = JSON.stringify(preset.config);
      setActivePreset(preset.id);
      onReset(undefined, configStr);
      addToast(
        `Scenario: ${preset.name}`,
        '\u{1F9EA}', // test tube
        preset.color.replace('bg-', '').replace('-700', '').replace('-600', ''),
      );
    },
    [onReset, addToast],
  );

  return (
    <div className="border-b border-gray-700">
      <button
        onClick={() => setExpanded(!expanded)}
        className="w-full flex items-center justify-between px-3 py-2 text-xs font-bold text-gray-400 uppercase tracking-wider hover:text-gray-200 transition-colors"
      >
        <span>Scenario Presets</span>
        <span className="text-gray-500">{expanded ? '\u25B2' : '\u25BC'}</span>
      </button>

      {expanded && (
        <div className="px-3 pb-3 space-y-2">
          <p className="text-[10px] text-gray-500 mb-2">
            Load pre-configured scenarios based on thesis findings. Each resets
            the simulation with specific parameters.
          </p>
          <div className="grid grid-cols-2 gap-1.5">
            {SCENARIO_PRESETS.map((preset) => (
              <button
                key={preset.id}
                onClick={() => handlePreset(preset)}
                className={`group relative px-2 py-1.5 rounded text-[10px] text-white text-left transition-all ${
                  preset.color
                } ${
                  activePreset === preset.id
                    ? 'ring-1 ring-white/40'
                    : 'opacity-80 hover:opacity-100'
                }`}
                title={preset.description}
              >
                <span className="font-medium block leading-tight">
                  {preset.name}
                </span>

                {/* Tooltip on hover */}
                <div className="absolute z-20 left-0 bottom-full mb-1 w-56 px-2 py-1.5 bg-gray-900 border border-gray-600 rounded text-[9px] text-gray-300 opacity-0 group-hover:opacity-100 pointer-events-none transition-opacity leading-snug">
                  {preset.description}
                </div>
              </button>
            ))}
          </div>

          {activePreset && (
            <div className="mt-2 px-2 py-1 bg-gray-700/50 rounded">
              <p className="text-[9px] text-gray-400">
                Active:{' '}
                <span className="text-gray-200">
                  {SCENARIO_PRESETS.find((p) => p.id === activePreset)?.name}
                </span>
              </p>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
