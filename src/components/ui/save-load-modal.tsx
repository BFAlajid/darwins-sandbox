'use client';

import { useState, useEffect, useCallback } from 'react';
import { useSthStore } from '@/stores/sth-store';
import { listSaves, saveSim, deleteSave, generateShareUrl, type SaveSlot } from '@/lib/save-system';

interface Props {
  onLoadSeed: (seed: number) => void;
}

export function SaveLoadModal({ onLoadSeed }: Props) {
  const show = useSthStore((s) => s.showSaveLoad);
  const stats = useSthStore((s) => s.stats);
  const [saves, setSaves] = useState<SaveSlot[]>([]);
  const [saveName, setSaveName] = useState('');
  const [copied, setCopied] = useState(false);

  const refreshSaves = useCallback(async () => {
    try {
      const list = await listSaves();
      setSaves(list);
    } catch { /* IndexedDB not available */ }
  }, []);

  useEffect(() => {
    if (show) refreshSaves();
  }, [show, refreshSaves]);

  if (!show) return null;

  const currentSeed = 42;

  const handleSave = async () => {
    if (!stats) return;
    const name = saveName.trim() || `Save ${new Date().toLocaleString()}`;
    const slot: SaveSlot = {
      id: crypto.randomUUID(),
      name,
      seed: currentSeed,
      config: {},
      timestamp: Date.now(),
      tick: stats.tick,
      agentCount: stats.agentCount,
      prevalenceAny: stats.prevalenceAny,
    };
    await saveSim(slot);
    setSaveName('');
    refreshSaves();
  };

  const handleDelete = async (id: string) => {
    await deleteSave(id);
    refreshSaves();
  };

  const handleLoad = (seed: number) => {
    onLoadSeed(seed);
    useSthStore.getState().setShowSaveLoad(false);
  };

  const handleShare = async () => {
    const url = generateShareUrl(currentSeed);
    try {
      await navigator.clipboard.writeText(url);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      window.prompt('Copy this URL:', url);
    }
  };

  const formatDate = (ts: number) => {
    const d = new Date(ts);
    return d.toLocaleDateString() + ' ' + d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60"
      onClick={() => useSthStore.getState().setShowSaveLoad(false)}
    >
      <div
        className="bg-gray-800 border border-gray-700 rounded-lg p-5 max-w-lg w-full shadow-xl max-h-[80vh] flex flex-col"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-gray-200 font-bold text-sm">Save / Load</h2>
          <button
            onClick={() => useSthStore.getState().setShowSaveLoad(false)}
            className="text-gray-500 hover:text-gray-300 text-xs"
          >
            [Esc]
          </button>
        </div>

        {/* Save current */}
        <div className="flex gap-2 mb-4">
          <input
            type="text"
            placeholder="Save name (optional)"
            value={saveName}
            onChange={(e) => setSaveName(e.target.value)}
            className="flex-1 px-2 py-1.5 bg-gray-900 border border-gray-600 rounded text-xs text-gray-200 placeholder-gray-600 outline-none focus:border-emerald-500"
            onKeyDown={(e) => e.key === 'Enter' && handleSave()}
          />
          <button
            onClick={handleSave}
            disabled={!stats}
            className="px-3 py-1.5 rounded bg-emerald-600 text-white text-xs font-medium hover:opacity-90 disabled:opacity-40"
          >
            Save
          </button>
        </div>

        {/* Share URL */}
        <div className="flex gap-2 mb-4">
          <button
            onClick={handleShare}
            className="flex-1 px-3 py-1.5 rounded bg-gray-700 text-gray-300 text-xs hover:bg-gray-600 text-center"
          >
            {copied ? 'Copied!' : 'Copy Seed URL'}
          </button>
        </div>

        {/* Saved list */}
        <div className="flex-1 overflow-y-auto">
          {saves.length === 0 ? (
            <p className="text-gray-600 text-xs text-center py-4">No saves yet</p>
          ) : (
            <div className="flex flex-col gap-1">
              {saves.map((s) => (
                <div
                  key={s.id}
                  className="flex items-center gap-2 p-2 rounded bg-gray-900 hover:bg-gray-700 group"
                >
                  <div className="flex-1 min-w-0">
                    <div className="text-gray-200 text-xs font-medium truncate">{s.name}</div>
                    <div className="text-gray-500 text-[10px] font-mono">
                      Tick {s.tick.toLocaleString()} | {s.agentCount} agents | STH {((s.prevalenceAny ?? 0) * 100).toFixed(1)}%
                    </div>
                    <div className="text-gray-600 text-[10px]">{formatDate(s.timestamp)}</div>
                  </div>
                  <button
                    onClick={() => handleLoad(s.seed)}
                    className="px-2 py-1 rounded bg-emerald-600 text-white text-[10px] font-medium hover:opacity-90"
                  >
                    Load
                  </button>
                  <button
                    onClick={() => handleDelete(s.id)}
                    className="px-2 py-1 rounded bg-gray-700 text-gray-500 text-[10px] hover:bg-red-900 hover:text-red-200 opacity-0 group-hover:opacity-100 transition-opacity"
                  >
                    Del
                  </button>
                </div>
              ))}
            </div>
          )}
        </div>

        {/* Presets */}
        <div className="mt-4 pt-3 border-t border-gray-700">
          <div className="text-gray-500 text-[10px] mb-2 uppercase tracking-wider">Preset Seeds</div>
          <div className="flex gap-2 flex-wrap">
            {[
              { seed: 42, label: 'Default' },
              { seed: 1337, label: 'Cebu A' },
              { seed: 2024, label: 'Cebu B' },
              { seed: 7777, label: 'High Prev' },
              { seed: 314159, label: 'Pi' },
            ].map((p) => (
              <button
                key={p.seed}
                onClick={() => handleLoad(p.seed)}
                className="px-2.5 py-1 rounded bg-gray-700 text-gray-400 text-[10px] hover:text-gray-200 hover:bg-gray-600"
              >
                {p.label} ({p.seed})
              </button>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}
