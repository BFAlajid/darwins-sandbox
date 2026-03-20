'use client';

import { useEffect, useCallback } from 'react';
import { useSthStore } from '@/stores/sth-store';

interface KeyboardActions {
  play: () => void;
  pause: () => void;
  step: () => void;
  setSpeed: (speed: number) => void;
  reset: () => void;
}

export function useKeyboard(actions: KeyboardActions) {
  const handleKeyDown = useCallback((e: KeyboardEvent) => {
    // Ignore when typing in inputs
    if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;

    const store = useSthStore.getState();

    switch (e.key) {
      case ' ':
        e.preventDefault();
        store.state === 'running' ? actions.pause() : actions.play();
        break;
      case '.':
        if (store.state !== 'running') actions.step();
        break;
      case '1': actions.setSpeed(1); break;
      case '2': actions.setSpeed(2); break;
      case '3': actions.setSpeed(5); break;
      case '4': actions.setSpeed(10); break;
      case '5': actions.setSpeed(50); break;
      case 'c':
      case 'C':
        store.cycleDataLens();
        break;
      case 'v':
      case 'V':
        store.toggleContaminationHeatmap();
        break;
      case 'f':
      case 'F':
        store.toggleFacilities();
        break;
      case 'm':
      case 'M':
        store.toggleMinimap();
        break;
      case 't':
      case 'T':
        store.toggleAgentPaths();
        break;
      case 'p':
      case 'P':
        store.setShowParams(!store.showParams);
        break;
      case 'h':
      case 'H':
        store.setShowHelp(!store.showHelp);
        break;
      case 's':
      case 'S':
        store.setShowSaveLoad(!store.showSaveLoad);
        break;
      case 'r':
      case 'R':
        if (e.ctrlKey || e.metaKey) return; // don't hijack browser refresh
        actions.reset();
        break;
      case 'Escape':
        if (store.facilityPlacementMode !== null) {
          store.setFacilityPlacementMode(null);
        } else if (store.showSaveLoad) {
          store.setShowSaveLoad(false);
        } else if (store.showHelp) {
          store.setShowHelp(false);
        } else if (store.showParams) {
          store.setShowParams(false);
        } else if (store.selectedAgent !== null) {
          store.selectAgent(null);
        }
        break;
    }
  }, [actions]);

  useEffect(() => {
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [handleKeyDown]);
}
