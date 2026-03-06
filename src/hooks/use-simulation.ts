'use client';

import { useCallback, useEffect, useRef } from 'react';
import type { SimCommand, WorkerMessage } from '@/lib/types';
import { useSimulationStore } from '@/stores/simulation-store';

const WATCHDOG_TIMEOUT = 5000;
const STATS_THROTTLE_MS = 200; // 5Hz

export function useSimulation() {
  const workerRef = useRef<Worker | null>(null);
  const creatureBufferRef = useRef<Float32Array | null>(null);
  const foodBufferRef = useRef<Float32Array | null>(null);
  const lastHeartbeatRef = useRef<number>(Date.now());
  const watchdogRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const lastStatsUpdateRef = useRef<number>(0);

  const store = useSimulationStore();

  const postCommand = useCallback((cmd: SimCommand) => {
    workerRef.current?.postMessage(cmd);
  }, []);

  const initWorker = useCallback(() => {
    if (workerRef.current) {
      workerRef.current.terminate();
    }

    store.setState('loading');

    const worker = new Worker(
      new URL('../workers/simulation.worker.ts', import.meta.url),
      { type: 'module' }
    );

    worker.onmessage = (e: MessageEvent<WorkerMessage>) => {
      const msg = e.data;

      switch (msg.type) {
        case 'ready':
          store.setState('paused');
          break;

        case 'frame': {
          // Store render data in refs (never in React state)
          creatureBufferRef.current = new Float32Array(msg.creatureBuffer);
          foodBufferRef.current = new Float32Array(msg.foodBuffer);

          // Throttle stats updates to 5Hz
          const now = Date.now();
          if (now - lastStatsUpdateRef.current > STATS_THROTTLE_MS) {
            store.setStats(msg.stats);
            store.setWorldSize(msg.worldWidth, msg.worldHeight);
            lastStatsUpdateRef.current = now;
          }
          break;
        }

        case 'heartbeat':
          lastHeartbeatRef.current = Date.now();
          break;

        case 'error':
          store.setError(msg.message);
          break;
      }
    };

    worker.onerror = (e) => {
      store.setError(e.message || 'Worker crashed');
    };

    workerRef.current = worker;

    // Initialize WASM
    worker.postMessage({ type: 'init' } as SimCommand);

    // Start watchdog
    if (watchdogRef.current) clearInterval(watchdogRef.current);
    watchdogRef.current = setInterval(() => {
      if (
        store.state === 'running' &&
        Date.now() - lastHeartbeatRef.current > WATCHDOG_TIMEOUT
      ) {
        console.warn('Worker watchdog: no heartbeat, restarting...');
        initWorker();
      }
    }, 1000);
  }, [store]);

  const play = useCallback(() => {
    const speed = useSimulationStore.getState().speed;
    store.setState('running');
    postCommand({ type: 'step', count: speed });
  }, [store, postCommand]);

  const pause = useCallback(() => {
    store.setState('paused');
    postCommand({ type: 'pause' });
  }, [store, postCommand]);

  const step = useCallback(() => {
    postCommand({ type: 'step', count: 1 });
  }, [postCommand]);

  const reset = useCallback((seed?: number) => {
    store.setState('loading');
    postCommand({ type: 'reset', seed });
  }, [store, postCommand]);

  const setSpeed = useCallback((speed: number) => {
    store.setSpeed(speed);
    if (useSimulationStore.getState().state === 'running') {
      postCommand({ type: 'pause' });
      postCommand({ type: 'step', count: speed });
    }
  }, [store, postCommand]);

  // Auto-init on mount
  useEffect(() => {
    initWorker();
    return () => {
      if (watchdogRef.current) clearInterval(watchdogRef.current);
      workerRef.current?.terminate();
    };
  }, [initWorker]);

  return {
    creatureBufferRef,
    foodBufferRef,
    play,
    pause,
    step,
    reset,
    setSpeed,
  };
}
