'use client';

import { useSthStore } from '@/stores/sth-store';

const SHORTCUTS: [string, string][] = [
  ['Space', 'Play / Pause'],
  ['.', 'Single step'],
  ['1-5', 'Speed presets (1x, 2x, 5x, 10x, 50x)'],
  ['C', 'Cycle data lens (infection/KAP/contamination/agents)'],
  ['V', 'Toggle contamination heatmap'],
  ['F', 'Toggle facility markers'],
  ['M', 'Toggle minimap'],
  ['T', 'Toggle agent path trails'],
  ['P', 'Toggle parameter panel'],
  ['S', 'Save / Load'],
  ['H', 'Toggle this help'],
  ['R', 'Reset simulation'],
  ['Esc', 'Cancel placement / close panel / deselect'],
];

const FEATURES: string[] = [
  'STH infection model (Ascaris, Trichuris, Hookworm)',
  'KAP behavioral model with structural gates',
  'Interventions: MDA, WASH, Education, BHW visits',
  'Urban vs Rural comparison mode',
  'Contamination heatmap overlay',
  'Click-to-place WASH facilities',
  'Day/night visual cycle',
  'Prevalence & KAP charts with thesis data overlay',
  'Budget tracking & cost analysis',
];

export function KeyboardHelp() {
  const show = useSthStore((s) => s.showHelp);
  if (!show) return null;

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60"
      onClick={() => useSthStore.getState().setShowHelp(false)}
    >
      <div
        className="bg-gray-800 border border-gray-700 rounded-lg p-6 max-w-sm w-full shadow-xl max-h-[80vh] overflow-y-auto"
        onClick={(e) => e.stopPropagation()}
        role="dialog"
        aria-label="Keyboard Shortcuts"
      >
        <h2 className="text-gray-200 font-bold text-sm mb-4">Keyboard Shortcuts</h2>
        <div className="flex flex-col gap-2">
          {SHORTCUTS.map(([key, desc]) => (
            <div key={key} className="flex items-center gap-3">
              <kbd className="bg-gray-900 px-2 py-0.5 rounded text-xs font-mono text-gray-300 min-w-[48px] text-center border border-gray-600">
                {key}
              </kbd>
              <span className="text-gray-400 text-xs">{desc}</span>
            </div>
          ))}
        </div>

        <h3 className="text-gray-300 font-bold text-xs mt-5 mb-2">Features</h3>
        <ul className="flex flex-col gap-1">
          {FEATURES.map((f) => (
            <li key={f} className="text-gray-500 text-[11px] pl-2 before:content-['-'] before:mr-1.5">{f}</li>
          ))}
        </ul>

        <p className="text-gray-600 text-xs mt-4 text-center">Press Esc or H to close</p>
      </div>
    </div>
  );
}
