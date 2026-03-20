import { create } from 'zustand';
import type {
  SimState,
  SthStats,
  SthDataLens,
  SthEvent,
  PrevalencePoint,
  InterventionEvent,
  FacilityType,
} from '@/lib/sth-types';

const MAX_EVENTS = 200;
const MAX_PREVALENCE_HISTORY = 500;
const LENS_ORDER: SthDataLens[] = ['infection', 'kap', 'contamination', 'agents'];

interface SthStore {
  // Simulation state
  state: SimState;
  stats: SthStats | null;
  speed: number;
  error: string | null;

  // World dimensions
  worldWidth: number;
  worldHeight: number;
  envGridWidth: number;
  envGridHeight: number;
  envCellSize: number;

  // Time
  currentTick: number;
  currentDay: number;
  currentMonth: number;

  // Map interaction
  selectedAgent: number | null;
  facilityPlacementMode: FacilityType | null;
  comparisonMode: boolean;

  // Comparison mode: second simulation stats
  stats2: SthStats | null;
  envGridWidth2: number;
  envGridHeight2: number;
  envCellSize2: number;
  prevalenceHistory2: PrevalencePoint[];

  // Data history
  prevalenceHistory: PrevalencePoint[];
  interventionLog: InterventionEvent[];
  budgetRemaining: number;
  budgetSpent: number;

  // UI toggles
  showContaminationHeatmap: boolean;
  showFacilities: boolean;
  showAgentPaths: boolean;
  dataLens: SthDataLens;
  showMinimap: boolean;
  showHelp: boolean;
  showParams: boolean;
  showSaveLoad: boolean;

  // Events
  events: SthEvent[];

  // Actions
  setState: (state: SimState) => void;
  setStats: (stats: SthStats) => void;
  setStats2: (stats: SthStats) => void;
  setSpeed: (speed: number) => void;
  setError: (error: string | null) => void;
  setWorldSize: (w: number, h: number) => void;
  setEnvGrid: (width: number, height: number, cellSize: number) => void;
  setEnvGrid2: (width: number, height: number, cellSize: number) => void;
  selectAgent: (index: number | null) => void;
  setFacilityPlacementMode: (mode: FacilityType | null) => void;
  setComparisonMode: (enabled: boolean) => void;
  toggleComparisonMode: () => void;
  addPrevalencePoint: (point: PrevalencePoint) => void;
  addPrevalencePoint2: (point: PrevalencePoint) => void;
  addInterventionEvent: (event: InterventionEvent) => void;
  toggleContaminationHeatmap: () => void;
  toggleFacilities: () => void;
  toggleMinimap: () => void;
  toggleAgentPaths: () => void;
  setDataLens: (lens: SthDataLens) => void;
  cycleDataLens: () => void;
  appendEvents: (newEvents: SthEvent[]) => void;
  clearHistory: () => void;
  setShowHelp: (show: boolean) => void;
  setShowParams: (show: boolean) => void;
  setShowSaveLoad: (show: boolean) => void;
  reset: () => void;
}

const initialState = {
  state: 'loading' as SimState,
  stats: null as SthStats | null,
  speed: 3,
  error: null as string | null,
  worldWidth: 800,
  worldHeight: 600,
  envGridWidth: 0,
  envGridHeight: 0,
  envCellSize: 10,
  currentTick: 0,
  currentDay: 0,
  currentMonth: 0,
  selectedAgent: null as number | null,
  facilityPlacementMode: null as FacilityType | null,
  comparisonMode: false,
  stats2: null as SthStats | null,
  envGridWidth2: 0,
  envGridHeight2: 0,
  envCellSize2: 10,
  prevalenceHistory2: [] as PrevalencePoint[],
  prevalenceHistory: [] as PrevalencePoint[],
  interventionLog: [] as InterventionEvent[],
  budgetRemaining: 0,
  budgetSpent: 0,
  showContaminationHeatmap: false,
  showFacilities: true,
  showMinimap: true,
  showAgentPaths: false,
  dataLens: 'infection' as SthDataLens,
  showHelp: false,
  showParams: false,
  showSaveLoad: false,
  events: [] as SthEvent[],
};

export const useSthStore = create<SthStore>((set) => ({
  ...initialState,

  setState: (state) => set({ state, error: null }),

  setStats: (stats) =>
    set({
      stats,
      currentTick: stats.tick,
      currentDay: stats.day,
      currentMonth: stats.month,
      budgetRemaining: stats.budgetRemaining,
      budgetSpent: stats.budgetSpent,
    }),

  setStats2: (stats) => set({ stats2: stats }),

  setSpeed: (speed) => set({ speed }),

  setError: (error) => set(error ? { error, state: 'error' } : { error: null }),

  setWorldSize: (worldWidth, worldHeight) => set({ worldWidth, worldHeight }),

  setEnvGrid: (envGridWidth, envGridHeight, envCellSize) =>
    set({ envGridWidth, envGridHeight, envCellSize }),

  setEnvGrid2: (envGridWidth2, envGridHeight2, envCellSize2) =>
    set({ envGridWidth2, envGridHeight2, envCellSize2 }),

  selectAgent: (selectedAgent) => set({ selectedAgent }),

  setFacilityPlacementMode: (facilityPlacementMode) => set({ facilityPlacementMode }),

  setComparisonMode: (comparisonMode) =>
    set({
      comparisonMode,
      // Clear sim2 state when exiting comparison mode
      ...(comparisonMode
        ? {}
        : { stats2: null, prevalenceHistory2: [] }),
    }),

  toggleComparisonMode: () =>
    set((s) => {
      const next = !s.comparisonMode;
      return {
        comparisonMode: next,
        ...(next ? {} : { stats2: null, prevalenceHistory2: [] }),
      };
    }),

  addPrevalencePoint: (point) =>
    set((s) => {
      const history = [...s.prevalenceHistory, point];
      return {
        prevalenceHistory: history.length > MAX_PREVALENCE_HISTORY
          ? history.slice(-MAX_PREVALENCE_HISTORY)
          : history,
      };
    }),

  addPrevalencePoint2: (point) =>
    set((s) => {
      const history = [...s.prevalenceHistory2, point];
      return {
        prevalenceHistory2: history.length > MAX_PREVALENCE_HISTORY
          ? history.slice(-MAX_PREVALENCE_HISTORY)
          : history,
      };
    }),

  addInterventionEvent: (event) =>
    set((s) => {
      const log = [...s.interventionLog, event];
      return {
        interventionLog: log.length > MAX_PREVALENCE_HISTORY
          ? log.slice(-MAX_PREVALENCE_HISTORY)
          : log,
      };
    }),

  toggleContaminationHeatmap: () =>
    set((s) => ({ showContaminationHeatmap: !s.showContaminationHeatmap })),

  toggleFacilities: () => set((s) => ({ showFacilities: !s.showFacilities })),

  toggleMinimap: () => set((s) => ({ showMinimap: !s.showMinimap })),

  toggleAgentPaths: () => set((s) => ({ showAgentPaths: !s.showAgentPaths })),

  setDataLens: (dataLens) => set({ dataLens }),

  cycleDataLens: () =>
    set((s) => {
      const idx = LENS_ORDER.indexOf(s.dataLens);
      return { dataLens: LENS_ORDER[(idx + 1) % LENS_ORDER.length] };
    }),

  appendEvents: (newEvents) =>
    set((s) => {
      const combined = [...s.events, ...newEvents];
      return { events: combined.slice(-MAX_EVENTS) };
    }),

  clearHistory: () =>
    set({
      prevalenceHistory: [],
      prevalenceHistory2: [],
      interventionLog: [],
      events: [],
    }),

  setShowHelp: (showHelp) => set({ showHelp }),
  setShowParams: (showParams) => set({ showParams }),
  setShowSaveLoad: (showSaveLoad) => set({ showSaveLoad }),

  reset: () => set({ ...initialState }),
}));
