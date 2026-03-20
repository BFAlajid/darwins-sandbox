'use client';

import { useState } from 'react';
import { useSthStore } from '@/stores/sth-store';

interface ParamDef {
  key: string;
  label: string;
  min: number;
  max: number;
  step: number;
  default: number;
  unit?: string;
  tooltip?: string;
}

const PARAMS: ParamDef[] = [
  { key: 'ascaris_lambda', label: 'Ascaris Transmission', min: 0.0001, max: 0.005, step: 0.0001, default: 0.0005, tooltip: 'Base transmission rate for Ascaris lumbricoides' },
  { key: 'trichuris_lambda', label: 'Trichuris Transmission', min: 0.0001, max: 0.005, step: 0.0001, default: 0.0006, tooltip: 'Base transmission rate for Trichuris trichiura' },
  { key: 'hookworm_lambda', label: 'Hookworm Transmission', min: 0.00001, max: 0.001, step: 0.00001, default: 0.00003, tooltip: 'Base transmission rate for hookworm' },
  { key: 'contamination_decay_rate', label: 'Contamination Decay', min: 0.001, max: 0.1, step: 0.001, default: 0.02, unit: '/tick', tooltip: 'Rate of natural soil contamination decay' },
  { key: 'rain_frequency', label: 'Rain Frequency', min: 0.0, max: 1.0, step: 0.05, default: 0.3, tooltip: 'Daily probability of rain event spreading contamination' },
  { key: 'rain_contamination_spread', label: 'Rain Spread Factor', min: 0.0, max: 1.0, step: 0.05, default: 0.3, tooltip: 'How much rain spreads contamination to adjacent cells' },
  { key: 'open_defecation_contamination', label: 'Open Defecation Load', min: 0.01, max: 0.5, step: 0.01, default: 0.1, tooltip: 'Contamination deposited per open defecation event' },
  { key: 'mda_cost_per_child', label: 'MDA Cost/Child', min: 0.5, max: 20, step: 0.5, default: 1, unit: ' units', tooltip: 'Cost per child for mass drug administration' },
  { key: 'latrine_cost', label: 'Latrine Cost', min: 10, max: 200, step: 5, default: 50, unit: ' units' },
  { key: 'total_budget', label: 'Starting Budget', min: 100, max: 10000, step: 100, default: 1000, unit: ' units' },
  { key: 'monthly_budget_increment', label: 'Monthly Income', min: 0, max: 500, step: 10, default: 100, unit: ' units' },
];

interface Preset {
  name: string;
  desc: string;
  values: Record<string, number>;
}

const PRESETS: Preset[] = [
  {
    name: 'Default',
    desc: 'Calibrated to thesis baseline (11.7% prevalence)',
    values: Object.fromEntries(PARAMS.map((p) => [p.key, p.default])),
  },
  {
    name: 'High Transmission',
    desc: 'Dense urban setting, poor sanitation, high reinfection',
    values: {
      ascaris_lambda: 0.002, trichuris_lambda: 0.0015, hookworm_lambda: 0.0001,
      contamination_decay_rate: 0.01, rain_frequency: 0.5, rain_contamination_spread: 0.5,
      open_defecation_contamination: 0.2, mda_cost_per_child: 1,
      latrine_cost: 50, total_budget: 1000, monthly_budget_increment: 100,
    },
  },
  {
    name: 'Low WASH',
    desc: 'Minimal sanitation infrastructure, high open defecation',
    values: {
      ascaris_lambda: 0.001, trichuris_lambda: 0.001, hookworm_lambda: 0.00005,
      contamination_decay_rate: 0.005, rain_frequency: 0.4, rain_contamination_spread: 0.4,
      open_defecation_contamination: 0.3, mda_cost_per_child: 1,
      latrine_cost: 50, total_budget: 500, monthly_budget_increment: 50,
    },
  },
  {
    name: 'Well-Funded',
    desc: 'Large budget, standard transmission — test intervention strategies',
    values: {
      ascaris_lambda: 0.0005, trichuris_lambda: 0.0006, hookworm_lambda: 0.00003,
      contamination_decay_rate: 0.02, rain_frequency: 0.3, rain_contamination_spread: 0.3,
      open_defecation_contamination: 0.1, mda_cost_per_child: 1,
      latrine_cost: 50, total_budget: 5000, monthly_budget_increment: 300,
    },
  },
  {
    name: 'Dry Season',
    desc: 'No rain, slow contamination spread, faster decay',
    values: {
      ascaris_lambda: 0.0003, trichuris_lambda: 0.0004, hookworm_lambda: 0.00002,
      contamination_decay_rate: 0.04, rain_frequency: 0.05, rain_contamination_spread: 0.1,
      open_defecation_contamination: 0.1, mda_cost_per_child: 1,
      latrine_cost: 50, total_budget: 1000, monthly_budget_increment: 100,
    },
  },
];

interface Props {
  onReset: (seed?: number, config?: string) => void;
}

export function ParameterPanel({ onReset }: Props) {
  const show = useSthStore((s) => s.showParams);
  const [values, setValues] = useState<Record<string, number>>(() => {
    const v: Record<string, number> = {};
    for (const p of PARAMS) v[p.key] = p.default;
    return v;
  });
  const [dirty, setDirty] = useState(false);

  if (!show) return null;

  const handleChange = (key: string, val: number) => {
    setValues((prev) => ({ ...prev, [key]: val }));
    setDirty(true);
  };

  const handleApply = () => {
    const configJson = JSON.stringify(values);
    onReset(undefined, configJson);
    setDirty(false);
    useSthStore.getState().setShowParams(false);
  };

  const handlePreset = (preset: Preset) => {
    setValues({ ...preset.values });
    setDirty(true);
  };

  const handleResetDefaults = () => {
    const v: Record<string, number> = {};
    for (const p of PARAMS) v[p.key] = p.default;
    setValues(v);
    setDirty(true);
  };

  const formatValue = (p: ParamDef, val: number) => {
    if (p.step < 0.001) return val.toFixed(5);
    if (p.step < 0.01) return val.toFixed(4);
    if (p.step < 0.1) return val.toFixed(3);
    if (p.step < 1) return val.toFixed(2);
    return val.toFixed(0);
  };

  return (
    <div
      className="fixed inset-0 z-40 flex items-start justify-center pt-16 bg-black/40"
      onClick={() => useSthStore.getState().setShowParams(false)}
    >
      <div
        className="bg-gray-800 border border-gray-700 rounded-lg p-4 max-w-md w-full shadow-xl max-h-[70vh] overflow-y-auto"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center justify-between mb-3">
          <h2 className="text-gray-200 font-bold text-sm">STH Simulation Parameters</h2>
          <button
            onClick={() => useSthStore.getState().setShowParams(false)}
            className="text-gray-500 hover:text-gray-300 text-xs"
          >
            [Esc]
          </button>
        </div>

        {/* Preset buttons */}
        <div className="mb-3">
          <div className="text-gray-500 text-[10px] uppercase tracking-wider mb-1.5">Presets</div>
          <div className="flex flex-wrap gap-1.5">
            {PRESETS.map((preset) => (
              <button
                key={preset.name}
                onClick={() => handlePreset(preset)}
                title={preset.desc}
                className="px-2 py-1 rounded text-xs bg-gray-700 hover:bg-gray-600 text-gray-300 hover:text-gray-100 transition-colors"
              >
                {preset.name}
              </button>
            ))}
          </div>
        </div>

        <div className="flex flex-col gap-3">
          {PARAMS.map((p) => (
            <div key={p.key}>
              <div className="flex justify-between mb-0.5">
                <label className="text-gray-400 text-xs" title={p.tooltip}>{p.label}</label>
                <span className="text-gray-300 text-xs font-mono">
                  {formatValue(p, values[p.key] ?? p.default)}{p.unit ?? ''}
                </span>
              </div>
              <input
                type="range"
                min={p.min}
                max={p.max}
                step={p.step}
                value={values[p.key] ?? p.default}
                onChange={(e) => handleChange(p.key, parseFloat(e.target.value))}
                className="w-full h-1.5 bg-gray-900 rounded-full appearance-none cursor-pointer accent-emerald-500"
              />
            </div>
          ))}
        </div>

        <div className="flex gap-2 mt-4">
          <button
            onClick={handleApply}
            disabled={!dirty}
            className="flex-1 px-3 py-1.5 rounded bg-emerald-600 text-white text-xs font-medium hover:opacity-90 disabled:opacity-40"
          >
            Apply & Reset
          </button>
          <button
            onClick={handleResetDefaults}
            className="px-3 py-1.5 rounded bg-gray-700 text-gray-300 text-xs hover:bg-gray-600"
          >
            Defaults
          </button>
        </div>
        <p className="text-gray-600 text-[10px] mt-2 text-center">
          Changes reset the simulation with new parameters. Press [P] to toggle.
        </p>
      </div>
    </div>
  );
}
