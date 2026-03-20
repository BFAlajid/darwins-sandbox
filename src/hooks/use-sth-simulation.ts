'use client';

import { useCallback, useEffect, useRef } from 'react';
import type {
  SthCommand,
  SthWorkerMessage,
  SthStats,
  SthEvent,
  AgentDetail,
  PrevalencePoint,
  InterventionEvent,
} from '@/lib/sth-types';
import { useSthStore } from '@/stores/sth-store';

const WATCHDOG_TIMEOUT = 5000;
const STATS_THROTTLE_MS = 200; // 5Hz
const PREVALENCE_SAMPLE_INTERVAL_DAYS = 7; // Record prevalence history weekly

export function useSthSimulation() {
  const workerRef = useRef<Worker | null>(null);
  const agentBufferRef = useRef<Float32Array | null>(null);
  const envBufferRef = useRef<Float32Array | null>(null);
  const facilityBufferRef = useRef<Float32Array | null>(null);
  // Comparison mode: second simulation buffers
  const agentBufferRef2 = useRef<Float32Array | null>(null);
  const envBufferRef2 = useRef<Float32Array | null>(null);
  const facilityBufferRef2 = useRef<Float32Array | null>(null);

  const lastHeartbeatRef = useRef<number>(Date.now());
  const watchdogRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const lastStatsUpdateRef = useRef<number>(0);
  const lastPrevalenceDayRef = useRef<number>(-1);
  const lastPrevalenceDayRef2 = useRef<number>(-1);

  // Use getState() for imperative access -- do NOT subscribe to the store
  // (subscribing causes re-renders which recreate initWorker and restart the worker)
  const store = useSthStore.getState;

  const postCommand = useCallback(
    (cmd: SthCommand) => {
      workerRef.current?.postMessage(cmd);
    },
    [],
  );

  const initWorker = useCallback(() => {
    if (workerRef.current) {
      workerRef.current.terminate();
    }

    store().setState('loading');

    const worker = new Worker(
      new URL('../workers/sth.worker.ts', import.meta.url),
      { type: 'module' },
    );

    worker.onmessage = (e: MessageEvent<SthWorkerMessage>) => {
      const msg = e.data;

      switch (msg.type) {
        case 'ready':
          store().setState('paused');
          if (msg.worldWidth && msg.worldHeight) {
            store().setWorldSize(msg.worldWidth, msg.worldHeight);
          }
          break;

        case 'frame': {
          // Store render data in refs (never in React state)
          // IMPORTANT: .slice() to COPY data before transferring buffer back
          agentBufferRef.current = new Float32Array(msg.agentBuffer).slice();
          envBufferRef.current = new Float32Array(msg.envBuffer).slice();
          facilityBufferRef.current = new Float32Array(msg.facilityBuffer).slice();

          // Handle comparison mode second simulation data
          const returnPayload: any = {
            returnBuffers: true,
            agentBuffer: msg.agentBuffer,
            envBuffer: msg.envBuffer,
            facilityBuffer: msg.facilityBuffer,
          };
          const transferList: ArrayBuffer[] = [msg.agentBuffer, msg.envBuffer, msg.facilityBuffer];

          if (msg.agentBuffer2 && msg.envBuffer2 && msg.facilityBuffer2) {
            agentBufferRef2.current = new Float32Array(msg.agentBuffer2).slice();
            envBufferRef2.current = new Float32Array(msg.envBuffer2).slice();
            facilityBufferRef2.current = new Float32Array(msg.facilityBuffer2).slice();
            returnPayload.agentBuffer2 = msg.agentBuffer2;
            returnPayload.envBuffer2 = msg.envBuffer2;
            returnPayload.facilityBuffer2 = msg.facilityBuffer2;
            transferList.push(msg.agentBuffer2, msg.envBuffer2, msg.facilityBuffer2);
          } else {
            // Clear sim2 buffers when not in comparison mode
            agentBufferRef2.current = null;
            envBufferRef2.current = null;
            facilityBufferRef2.current = null;
          }

          // Return old buffers to worker for reuse (double-buffering)
          try {
            worker.postMessage(returnPayload, transferList);
          } catch {
            // Buffers already detached -- ok
          }

          // Update env grid dimensions
          store().setEnvGrid(msg.envGridWidth, msg.envGridHeight, msg.envCellSize);

          // Update sim2 env grid dimensions if present
          if (msg.envGridWidth2 !== undefined && msg.envGridHeight2 !== undefined && msg.envCellSize2 !== undefined) {
            store().setEnvGrid2(msg.envGridWidth2, msg.envGridHeight2, msg.envCellSize2);
          }

          // Throttle stats updates to 5Hz
          const now = Date.now();
          if (now - lastStatsUpdateRef.current > STATS_THROTTLE_MS) {
            try {
              const stats: SthStats = JSON.parse(msg.stats);
              store().setStats(stats);

              // Sample prevalence history at weekly intervals
              if (
                stats.day > 0 &&
                stats.day - lastPrevalenceDayRef.current >= PREVALENCE_SAMPLE_INTERVAL_DAYS
              ) {
                lastPrevalenceDayRef.current = stats.day;
                const point: PrevalencePoint = {
                  day: stats.day,
                  ascaris: stats.prevalenceAscaris,
                  trichuris: stats.prevalenceTrichuris,
                  hookworm: stats.prevalenceHookworm,
                  any: stats.prevalenceAny,
                };
                store().addPrevalencePoint(point);
              }
            } catch {
              // Ignore parse errors
            }

            // Parse and store sim2 stats if present
            if (msg.stats2) {
              try {
                const stats2: SthStats = JSON.parse(msg.stats2);
                store().setStats2(stats2);

                if (
                  stats2.day > 0 &&
                  stats2.day - lastPrevalenceDayRef2.current >= PREVALENCE_SAMPLE_INTERVAL_DAYS
                ) {
                  lastPrevalenceDayRef2.current = stats2.day;
                  const point2: PrevalencePoint = {
                    day: stats2.day,
                    ascaris: stats2.prevalenceAscaris,
                    trichuris: stats2.prevalenceTrichuris,
                    hookworm: stats2.prevalenceHookworm,
                    any: stats2.prevalenceAny,
                  };
                  store().addPrevalencePoint2(point2);
                }
              } catch {
                // Ignore parse errors
              }
            }

            lastStatsUpdateRef.current = now;
          }

          // Always process events (don't lose any)
          if (msg.events && msg.events !== '[]') {
            try {
              const newEvents: SthEvent[] = JSON.parse(msg.events);
              if (newEvents.length > 0) {
                store().appendEvents(newEvents);

                // Log intervention events to the intervention timeline
                for (const evt of newEvents) {
                  if (isInterventionEvent(evt)) {
                    const interventionEvent: InterventionEvent = {
                      day: store().currentDay,
                      type: evt.eventType,
                      description: evt.message,
                      cost: 0, // Cost is tracked in stats
                    };
                    store().addInterventionEvent(interventionEvent);
                  }
                }
              }
            } catch {
              // Ignore parse errors
            }
          }

          // Process sim2 events
          if (msg.events2 && msg.events2 !== '[]') {
            try {
              const newEvents2: SthEvent[] = JSON.parse(msg.events2);
              if (newEvents2.length > 0) {
                // Prefix events with barangay label for clarity in the log
                const labeled = newEvents2.map((evt) => ({
                  ...evt,
                  message: `[Rural] ${evt.message}`,
                }));
                store().appendEvents(labeled);
              }
            } catch {
              // Ignore parse errors
            }
          }
          break;
        }

        case 'heartbeat':
          lastHeartbeatRef.current = Date.now();
          break;

        case 'agentDetail': {
          // Agent detail is available for the inspector panel
          break;
        }

        case 'error':
          store().setError(msg.message);
          break;
      }
    };

    worker.onerror = (e) => {
      store().setError(e.message || 'Worker crashed');
    };

    workerRef.current = worker;

    // Initialize WASM
    worker.postMessage({ type: 'init' } as SthCommand);

    // Start watchdog
    if (watchdogRef.current) clearInterval(watchdogRef.current);
    watchdogRef.current = setInterval(() => {
      if (
        store().state === 'running' &&
        Date.now() - lastHeartbeatRef.current > WATCHDOG_TIMEOUT
      ) {
        console.warn('STH worker watchdog: no heartbeat, restarting...');
        initWorker();
      }
    }, 1000);
  }, [store]);

  // --- Playback controls ---

  const play = useCallback(() => {
    const speed = store().speed;
    store().setState('running');
    postCommand({ type: 'step', count: speed });
  }, [store, postCommand]);

  const pause = useCallback(() => {
    store().setState('paused');
    postCommand({ type: 'pause' });
  }, [store, postCommand]);

  const step = useCallback(() => {
    // Send a single tick then immediately pause so the run loop doesn't continue
    postCommand({ type: 'step', count: 1 });
    // Use a microtask to ensure the step message is processed before the pause
    queueMicrotask(() => postCommand({ type: 'pause' }));
  }, [postCommand]);

  const reset = useCallback(
    (seed?: number, config?: string) => {
      store().reset();
      store().setState('loading');
      lastPrevalenceDayRef.current = -1;
      lastPrevalenceDayRef2.current = -1;
      const compMode = store().comparisonMode;
      postCommand({ type: 'reset', seed, config, comparisonMode: compMode });
    },
    [store, postCommand],
  );

  const setSpeed = useCallback(
    (speed: number) => {
      store().setSpeed(speed);
      if (store().state === 'running') {
        postCommand({ type: 'pause' });
        postCommand({ type: 'step', count: speed });
      }
    },
    [store, postCommand],
  );

  // --- Comparison mode ---

  const toggleComparisonMode = useCallback(() => {
    const current = store().comparisonMode;
    const next = !current;
    store().setComparisonMode(next);
    store().clearHistory();
    lastPrevalenceDayRef.current = -1;
    lastPrevalenceDayRef2.current = -1;
    postCommand({ type: 'toggleComparisonMode', enabled: next });
  }, [store, postCommand]);

  // --- Intervention commands ---

  const launchMDA = useCallback(
    (schoolId: number, drug: number) => {
      postCommand({ type: 'launchMDA', schoolId, drug });
    },
    [postCommand],
  );

  const buildLatrine = useCallback(
    (x: number, y: number) => {
      postCommand({ type: 'buildLatrine', x, y });
      store().setFacilityPlacementMode(null);
    },
    [postCommand, store],
  );

  const buildWaterPump = useCallback(
    (x: number, y: number) => {
      postCommand({ type: 'buildWaterPump', x, y });
      store().setFacilityPlacementMode(null);
    },
    [postCommand, store],
  );

  const buildHandwashStation = useCallback(
    (x: number, y: number) => {
      postCommand({ type: 'buildHandwashStation', x, y });
      store().setFacilityPlacementMode(null);
    },
    [postCommand, store],
  );

  const launchEducation = useCallback(
    (method: number, schoolId: number) => {
      postCommand({ type: 'launchEducation', method, schoolId });
    },
    [postCommand],
  );

  const launchBhwVisits = useCallback(
    (coverage: number) => {
      if (coverage < 0 || coverage > 1) {
        console.warn('BHW visit coverage must be between 0 and 1');
        return;
      }
      postCommand({ type: 'launchBhwVisits', coverage });
    },
    [postCommand],
  );

  const increaseBudget = useCallback(
    (multiplier: number) => {
      if (multiplier <= 0) {
        console.warn('Budget multiplier must be positive');
        return;
      }
      postCommand({ type: 'increaseBudget', multiplier });
    },
    [postCommand],
  );

  const selectAgent = useCallback(
    (index: number | null) => {
      store().selectAgent(index);
      postCommand({ type: 'selectAgent', index });
    },
    [store, postCommand],
  );

  // --- Lifecycle ---

  // Auto-init on mount
  useEffect(() => {
    initWorker();
    return () => {
      if (watchdogRef.current) clearInterval(watchdogRef.current);
      workerRef.current?.terminate();
    };
  }, [initWorker]);

  // Sync selected agent to worker when it changes externally
  useEffect(() => {
    let prev: number | null = null;
    return useSthStore.subscribe((s) => {
      if (s.selectedAgent !== prev) {
        prev = s.selectedAgent;
        postCommand({ type: 'selectAgent', index: s.selectedAgent });
      }
    });
  }, [postCommand]);

  return {
    // Render data refs
    agentBufferRef,
    envBufferRef,
    facilityBufferRef,
    // Comparison mode render data refs
    agentBufferRef2,
    envBufferRef2,
    facilityBufferRef2,

    // Playback
    play,
    pause,
    step,
    reset,
    setSpeed,

    // Comparison mode
    toggleComparisonMode,

    // Interventions
    launchMDA,
    buildLatrine,
    buildWaterPump,
    buildHandwashStation,
    launchEducation,
    launchBhwVisits,
    increaseBudget,

    // Selection
    selectAgent,
  };
}

/** Check if an event represents an intervention */
function isInterventionEvent(evt: SthEvent): boolean {
  const interventionTypes = [
    'mda_completed',
    'mda_launched',
    'latrine_built',
    'water_pump_built',
    'handwash_station_built',
    'education_session',
    'bhw_visit',
    'budget_increase',
  ];
  return interventionTypes.includes(evt.eventType);
}
