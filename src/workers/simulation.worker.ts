/// <reference lib="webworker" />

import type { SimCommand, WorkerMessage, SimStats } from '../lib/types';

let sim: any = null;
let wasmExports: any = null;
let running = false;
let timeoutId: ReturnType<typeof setTimeout> | null = null;

const TICK_BUDGET_MS = 14;

async function initWasm(): Promise<void> {
  // Fetch the JS glue as text, convert to blob URL for reliable import
  const [glueResponse, wasmResponse] = await Promise.all([
    fetch('/wasm/simulation.js'),
    fetch('/wasm/simulation_bg.wasm'),
  ]);

  const glueText = await glueResponse.text();
  const wasmBytes = await wasmResponse.arrayBuffer();

  // Create blob URL from the glue JS so we can import it as a module
  const blob = new Blob([glueText], { type: 'application/javascript' });
  const blobUrl = URL.createObjectURL(blob);

  try {
    const glue = await import(/* webpackIgnore: true */ blobUrl);
    await glue.default(wasmBytes);
    wasmExports = glue;

  } finally {
    URL.revokeObjectURL(blobUrl);
  }
}

function getStats(): SimStats {
  return {
    tick: Number(sim.get_tick()),
    creatureCount: sim.get_creature_count(),
    foodCount: sim.get_food_count(),
    generationMax: sim.get_generation_max(),
    totalBirths: Number(sim.get_total_births()),
    totalDeaths: Number(sim.get_total_deaths()),
    nanDeaths: sim.get_nan_deaths(),
    energyDriftPct: sim.get_energy_drift_pct(),
    profileJson: sim.get_profile_json(),
  };
}

function packAndSendFrame() {
  if (!sim) return;

  // Get food data (returned as Float32Array by wasm-bindgen)
  const foodData: Float32Array = sim.get_food_render_data();
  const foodBuffer = new ArrayBuffer(foodData.byteLength);
  new Float32Array(foodBuffer).set(foodData);

  // Get creature render data (wasm-bindgen copies it out for us)
  const creatureData: Float32Array = sim.get_creature_render_data();
  const creatureBuffer = new ArrayBuffer(creatureData.byteLength);
  new Float32Array(creatureBuffer).set(creatureData);

  const stats = getStats();

  const msg: WorkerMessage = {
    type: 'frame',
    creatureBuffer,
    foodBuffer,
    stats,
    worldWidth: sim.get_world_width(),
    worldHeight: sim.get_world_height(),
  };
  postMessage(msg, [creatureBuffer, foodBuffer] as any);
}

function runLoop(ticksPerFrame: number) {
  if (!running || !sim) return;

  const start = performance.now();
  let ticksDone = 0;

  while (ticksDone < ticksPerFrame && (performance.now() - start) < TICK_BUDGET_MS) {
    sim.step();
    ticksDone++;
  }

  packAndSendFrame();
  postMessage({ type: 'heartbeat' } as WorkerMessage);

  timeoutId = setTimeout(() => runLoop(ticksPerFrame), 0);
}

onmessage = async (e: MessageEvent<SimCommand>) => {
  const cmd = e.data;

  try {
    switch (cmd.type) {
      case 'init': {
        if (!wasmExports) {
          await initWasm();
        }
        const { Simulation } = wasmExports;
        if (cmd.seed !== undefined) {
          sim = Simulation.with_seed(BigInt(cmd.seed));
        } else {
          sim = new Simulation();
        }

        postMessage({ type: 'ready' } as WorkerMessage);
        break;
      }

      case 'step': {
        if (!sim) return;
        running = true;
        runLoop(cmd.count);
        break;
      }

      case 'pause': {
        running = false;
        if (timeoutId !== null) {
          clearTimeout(timeoutId);
          timeoutId = null;
        }
        break;
      }

      case 'reset': {
        running = false;
        if (timeoutId !== null) {
          clearTimeout(timeoutId);
          timeoutId = null;
        }
        if (wasmExports) {
          const { Simulation } = wasmExports;
          if (cmd.seed !== undefined) {
            sim = Simulation.with_seed(BigInt(cmd.seed));
          } else {
            sim = new Simulation();
          }
          packAndSendFrame();
        }
        postMessage({ type: 'ready' } as WorkerMessage);
        break;
      }
    }
  } catch (err: any) {
    postMessage({
      type: 'error',
      message: err?.message || String(err),
    } as WorkerMessage);
  }
};
