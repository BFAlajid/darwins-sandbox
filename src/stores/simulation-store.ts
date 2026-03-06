import { create } from 'zustand';
import type { SimStats } from '@/lib/types';

export type SimState = 'uninitialized' | 'loading' | 'running' | 'paused' | 'error';

interface SimulationStore {
  // State
  state: SimState;
  stats: SimStats | null;
  speed: number;
  error: string | null;
  worldWidth: number;
  worldHeight: number;

  // Actions
  setState: (state: SimState) => void;
  setStats: (stats: SimStats) => void;
  setSpeed: (speed: number) => void;
  setError: (error: string | null) => void;
  setWorldSize: (w: number, h: number) => void;
}

export const useSimulationStore = create<SimulationStore>((set) => ({
  state: 'uninitialized',
  stats: null,
  speed: 3,
  error: null,
  worldWidth: 800,
  worldHeight: 600,

  setState: (state) => set({ state, error: state === 'error' ? undefined : null }),
  setStats: (stats) => set({ stats }),
  setSpeed: (speed) => set({ speed }),
  setError: (error) => set({ error, state: 'error' }),
  setWorldSize: (worldWidth, worldHeight) => set({ worldWidth, worldHeight }),
}));
