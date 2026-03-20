'use client';

import { useEffect, useCallback, useRef, useState } from 'react';
import { SthCanvas } from '@/components/canvas/sth-canvas';
import { SthControls } from '@/components/controls/sth-controls';
import { InterventionToolbar } from '@/components/controls/intervention-toolbar';
import { ParameterPanel } from '@/components/controls/parameter-panel';
import { AgentInspector } from '@/components/sidebar/agent-inspector';
import { BarangayStats } from '@/components/sidebar/barangay-stats';
import { SthEventLog } from '@/components/sidebar/sth-event-log';
import { PrevalenceChart } from '@/components/charts/prevalence-chart';
import { KapChart } from '@/components/charts/kap-chart';
import { InterventionTimeline } from '@/components/charts/intervention-timeline';
import { CostTracker } from '@/components/charts/cost-tracker';
import { ScenarioPresets } from '@/components/controls/scenario-presets';
import { useSthSimulation } from '@/hooks/use-sth-simulation';
import { useKeyboard } from '@/hooks/use-keyboard';
import { useSthStore } from '@/stores/sth-store';
import { FacilityType } from '@/lib/sth-types';
import { THESIS_DATA } from '@/lib/thesis-data';
import {
  exportPrevalenceCsv,
  exportComparisonCsv,
  exportInterventionsCsv,
  exportStatsJson,
  captureScreenshot,
} from '@/lib/export';

export default function Home() {
  const {
    agentBufferRef,
    envBufferRef,
    facilityBufferRef,
    agentBufferRef2,
    envBufferRef2,
    facilityBufferRef2,
    play,
    pause,
    step,
    reset,
    setSpeed,
    toggleComparisonMode,
    launchMDA,
    buildLatrine,
    buildWaterPump,
    buildHandwashStation,
    launchEducation,
    launchBhwVisits,
    increaseBudget,
    selectAgent,
  } = useSthSimulation();

  const state = useSthStore((s) => s.state);
  const error = useSthStore((s) => s.error);
  const selectedAgent = useSthStore((s) => s.selectedAgent);
  const dataLens = useSthStore((s) => s.dataLens);
  const facilityPlacementMode = useSthStore((s) => s.facilityPlacementMode);
  const prevalenceAny = useSthStore((s) => s.stats?.prevalenceAny ?? 0);
  const [sidebarOpen, setSidebarOpen] = useState(true);

  // Outbreak alarm state (flashing when prevalence > 20%)
  const [outbreakFlash, setOutbreakFlash] = useState(false);
  const isOutbreak = prevalenceAny >= 0.2;

  useEffect(() => {
    if (!isOutbreak) {
      setOutbreakFlash(false);
      return;
    }
    const interval = setInterval(() => setOutbreakFlash((v) => !v), 800);
    return () => clearInterval(interval);
  }, [isOutbreak]);

  // Calibration mode: ?calibration=true in URL
  const [calibrationMode, setCalibrationMode] = useState(false);
  useEffect(() => {
    const params = new URLSearchParams(window.location.search);
    setCalibrationMode(params.get('calibration') === 'true');
  }, []);

  // Keyboard shortcuts
  useKeyboard({ play, pause, step, setSpeed, reset });

  // Auto-start after WASM loads (only once)
  const hasStarted = useRef(false);
  useEffect(() => {
    if (state === 'paused' && !hasStarted.current) {
      hasStarted.current = true;
      const timer = setTimeout(() => play(), 100);
      return () => clearTimeout(timer);
    }
  }, [state, play]);

  // Handle facility placement on canvas click
  const handleBuildFacility = useCallback(
    (type: FacilityType, x: number, y: number) => {
      switch (type) {
        case FacilityType.Latrine:
          buildLatrine(x, y);
          break;
        case FacilityType.WaterPump:
          buildWaterPump(x, y);
          break;
        case FacilityType.HandwashStation:
          buildHandwashStation(x, y);
          break;
      }
    },
    [buildLatrine, buildWaterPump, buildHandwashStation],
  );

  // Handle agent selection from canvas click
  const handleSelectAgent = useCallback(
    (index: number | null) => {
      selectAgent(index);
    },
    [selectAgent],
  );

  return (
    <div className="h-screen flex flex-col bg-gray-900 text-gray-100">
      {/* Top controls bar */}
      <SthControls
        onPlay={play}
        onPause={pause}
        onStep={step}
        onReset={reset}
        onSetSpeed={setSpeed}
        onToggleComparison={toggleComparisonMode}
      />

      {/* Intervention toolbar */}
      <InterventionToolbar
        onLaunchMDA={launchMDA}
        onBuildLatrine={buildLatrine}
        onBuildWaterPump={buildWaterPump}
        onBuildHandwashStation={buildHandwashStation}
        onLaunchEducation={launchEducation}
        onLaunchBhwVisits={launchBhwVisits}
        onIncreaseBudget={increaseBudget}
      />

      {/* Outbreak alarm banner */}
      {isOutbreak && (
        <div
          className={`px-4 py-1.5 text-center text-sm font-bold transition-colors ${
            outbreakFlash
              ? 'bg-red-600 text-white'
              : 'bg-red-900 text-red-200'
          }`}
        >
          OUTBREAK ALERT: STH prevalence exceeds 20% ({(prevalenceAny * 100).toFixed(1)}%) — Intervention recommended
        </div>
      )}

      {/* Main content: canvas + sidebar */}
      <div className="flex-1 flex flex-col md:flex-row overflow-hidden">
        {/* Canvas area */}
        <div className="flex-1 relative overflow-hidden min-h-[50vh] md:min-h-0">
          <SthCanvas
            agentBufferRef={agentBufferRef}
            envBufferRef={envBufferRef}
            facilityBufferRef={facilityBufferRef}
            agentBufferRef2={agentBufferRef2}
            envBufferRef2={envBufferRef2}
            facilityBufferRef2={facilityBufferRef2}
            onBuildFacility={handleBuildFacility}
            onSelectAgent={handleSelectAgent}
          />

          {/* Calibration mode badge */}
          {calibrationMode && (
            <div className="absolute top-10 left-1/2 -translate-x-1/2 pointer-events-none z-10">
              <span className="px-3 py-1 bg-purple-600/80 rounded text-xs text-white font-bold font-mono tracking-wider">
                CALIBRATION MODE
              </span>
            </div>
          )}

          {/* Data lens indicator */}
          <div className="absolute top-2 left-2 flex gap-2 pointer-events-none">
            <span className="px-2 py-1 bg-black/60 rounded text-xs text-gray-400 font-mono">
              Lens: {dataLens}
            </span>
            {facilityPlacementMode !== null && (
              <span className="px-2 py-1 bg-amber-600/80 rounded text-xs text-white font-mono">
                Click map to place facility
              </span>
            )}
          </div>

          {/* Lens toggle buttons */}
          <div className="absolute top-2 right-2 flex gap-1">
            {(['infection', 'kap', 'contamination', 'agents'] as const).map((lens) => (
              <button
                key={lens}
                onClick={() => useSthStore.getState().setDataLens(lens)}
                className={`px-2 py-1 rounded text-[10px] font-mono ${
                  dataLens === lens
                    ? 'bg-emerald-600 text-white'
                    : 'bg-black/60 text-gray-400 hover:text-gray-200'
                }`}
              >
                {lens}
              </button>
            ))}
          </div>

          {/* Keyboard shortcut hint (hidden on mobile) */}
          <div className="absolute bottom-2 left-2 pointer-events-none hidden md:block">
            <span className="px-2 py-1 bg-black/40 rounded text-[10px] text-gray-500 font-mono">
              [Space] Play/Pause  [P] Params  [C] Lens  [M] Minimap  [T] Trails  [V] Heatmap
            </span>
          </div>

          {/* Error overlay */}
          {error && (
            <div className="absolute inset-0 flex items-center justify-center bg-black/70">
              <div className="bg-gray-800 p-6 rounded-lg max-w-md text-center">
                <h2 className="text-red-400 text-lg font-bold mb-2">
                  Simulation Error
                </h2>
                <p className="text-gray-300 text-sm mb-4">{error}</p>
                <button
                  onClick={() => reset()}
                  className="px-4 py-2 bg-emerald-600 rounded text-white font-medium"
                >
                  Reset Simulation
                </button>
              </div>
            </div>
          )}

          {/* Loading state */}
          {state === 'loading' && (
            <div className="absolute inset-0 flex items-center justify-center bg-black/50">
              <p className="text-gray-400 text-sm">
                Loading STH simulation...
              </p>
            </div>
          )}
        </div>

        {/* Sidebar toggle (mobile only) */}
        <button
          onClick={() => setSidebarOpen((v) => !v)}
          className="md:hidden w-full py-2 bg-gray-800 border-t border-gray-700 text-gray-400 text-xs font-mono text-center active:bg-gray-700"
        >
          {sidebarOpen ? 'Hide Dashboard' : 'Show Dashboard'}
        </button>

        {/* Sidebar */}
        <div className={`w-full md:w-80 flex-shrink-0 bg-gray-800 border-t md:border-t-0 md:border-l border-gray-700 flex flex-col overflow-y-auto max-h-[50vh] md:max-h-none ${sidebarOpen ? '' : 'hidden md:flex'}`}>
          {/* Agent inspector (when selected) */}
          {selectedAgent !== null && (
            <div className="border-b border-gray-700">
              <div className="px-3 py-2 text-xs font-bold text-gray-400 uppercase tracking-wider">
                Inspector
              </div>
              <AgentInspector agentBufferRef={agentBufferRef} />
            </div>
          )}

          {/* Scenario presets */}
          <ScenarioPresets onReset={reset} />

          {/* Barangay stats */}
          <div className="border-b border-gray-700">
            <div className="px-3 py-2 text-xs font-bold text-gray-400 uppercase tracking-wider">
              STH Statistics
            </div>
            <BarangayStats />
          </div>

          {/* Calibration targets (only in calibration mode) */}
          {calibrationMode && (
            <div className="border-b border-gray-700 px-3 py-2">
              <div className="text-[10px] font-bold text-purple-400 uppercase tracking-wider mb-1.5">
                Thesis Targets
              </div>
              <div className="grid grid-cols-2 gap-x-3 gap-y-1 text-[10px] font-mono">
                <span className="text-gray-500">Overall STH</span>
                <span className="text-gray-300">{THESIS_DATA.prevalence.overall}%</span>
                <span className="text-gray-500">Urban STH</span>
                <span className="text-amber-400">{THESIS_DATA.prevalence.urban}%</span>
                <span className="text-gray-500">Rural STH</span>
                <span className="text-emerald-400">{THESIS_DATA.prevalence.rural}%</span>
                <span className="text-gray-500">Ascaris (overall)</span>
                <span className="text-gray-300">{THESIS_DATA.speciesPrevalence.ascaris.overall}%</span>
                <span className="text-gray-500">Trichuris (overall)</span>
                <span className="text-gray-300">{THESIS_DATA.speciesPrevalence.trichuris.overall}%</span>
                <span className="text-gray-500">Hookworm</span>
                <span className="text-gray-300">{THESIS_DATA.speciesPrevalence.hookworm.overall}%</span>
                <span className="text-gray-500">Urban Ascaris EPG</span>
                <span className="text-gray-300">{THESIS_DATA.meanEpg.ascaris.urban.toFixed(0)}</span>
                <span className="text-gray-500">Sample size</span>
                <span className="text-gray-300">{THESIS_DATA.sampleSize.total} (U:{THESIS_DATA.sampleSize.urban} R:{THESIS_DATA.sampleSize.rural})</span>
              </div>
            </div>
          )}

          {/* Charts */}
          <div className="border-b border-gray-700">
            <PrevalenceChart />
          </div>
          <div className="border-b border-gray-700">
            <KapChart />
          </div>
          <div className="border-b border-gray-700">
            <CostTracker />
          </div>
          <div className="border-b border-gray-700">
            <InterventionTimeline />
          </div>

          {/* Event log */}
          <div className="flex-1 flex flex-col min-h-0">
            <div className="px-3 py-2 text-xs font-bold text-gray-400 uppercase tracking-wider border-b border-gray-700">
              Event Log
            </div>
            <div className="flex-1 overflow-y-auto">
              <SthEventLog />
            </div>
          </div>

          {/* Export panel */}
          <div className="border-t border-gray-700 px-3 py-2">
            <div className="text-[10px] font-bold text-gray-400 uppercase tracking-wider mb-1.5">
              Export Data
            </div>
            <div className="flex flex-wrap gap-1.5">
              <button
                onClick={() => {
                  const s = useSthStore.getState();
                  if (s.comparisonMode) {
                    exportComparisonCsv(s.prevalenceHistory, s.prevalenceHistory2);
                  } else {
                    exportPrevalenceCsv(s.prevalenceHistory);
                  }
                }}
                className="px-2 py-1.5 rounded bg-gray-700 text-gray-300 text-[10px] hover:bg-gray-600 active:bg-gray-500"
              >
                Prevalence CSV
              </button>
              <button
                onClick={() => {
                  const s = useSthStore.getState();
                  exportInterventionsCsv(s.interventionLog);
                }}
                className="px-2 py-1.5 rounded bg-gray-700 text-gray-300 text-[10px] hover:bg-gray-600 active:bg-gray-500"
              >
                Interventions CSV
              </button>
              <button
                onClick={() => {
                  const s = useSthStore.getState();
                  if (s.stats) exportStatsJson(s.stats);
                }}
                className="px-2 py-1.5 rounded bg-gray-700 text-gray-300 text-[10px] hover:bg-gray-600 active:bg-gray-500"
              >
                Stats JSON
              </button>
              <button
                onClick={() => {
                  const canvas = document.querySelector('canvas');
                  if (canvas) {
                    const day = useSthStore.getState().currentDay;
                    captureScreenshot(canvas, `sth-day${day}.png`);
                  }
                }}
                className="px-2 py-1.5 rounded bg-gray-700 text-gray-300 text-[10px] hover:bg-gray-600 active:bg-gray-500"
              >
                Screenshot
              </button>
            </div>
          </div>
        </div>
      </div>

      {/* Parameter Panel Modal */}
      <ParameterPanel onReset={reset} />
    </div>
  );
}
