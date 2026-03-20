/// <reference lib="webworker" />

import type { SthCommand, SthWorkerMessage } from '../lib/sth-types';

let sim: any = null;
let sim2: any = null; // Second simulation for comparison mode
let wasmExports: any = null;
let running = false;
let comparisonMode = false;
let timeoutId: ReturnType<typeof setTimeout> | null = null;
let selectedAgentIdx: number | null = null;

const TICK_BUDGET_MS = 14;

// Double-buffer pools: reuse ArrayBuffers returned from main thread
let agentPoolBuf: ArrayBuffer | null = null;
let envPoolBuf: ArrayBuffer | null = null;
let facilityPoolBuf: ArrayBuffer | null = null;
// Pools for second simulation (comparison mode)
let agentPoolBuf2: ArrayBuffer | null = null;
let envPoolBuf2: ArrayBuffer | null = null;
let facilityPoolBuf2: ArrayBuffer | null = null;

// Default configs for comparison mode — nested in `barangays` array to match SthConfig
const URBAN_CONFIG = JSON.stringify({
  barangays: [{
    setting: 'Urban',
    name: 'Guadalupe',
    population_density: 5.0,
    latrine_coverage: 0.60,
    water_supply_coverage: 0.50,
    child_population: 300,
    num_households: 150,
    num_schools: 2,
    num_health_workers: 3,
    handwash_station_coverage: 0.30,
    open_defecation_rate: 0.30,
    baseline_soil_contamination: 0.08,
    medicine_supply_reliability: 0.70,
    bhw_per_thousand: 2.0,
    mean_knowledge: 0.55,
    mean_attitude: 0.65,
    mean_practice: 0.50,
  }],
});

const RURAL_CONFIG = JSON.stringify({
  barangays: [{
    setting: 'Rural',
    name: 'Sudlon II',
    population_density: 1.5,
    latrine_coverage: 0.75,
    water_supply_coverage: 0.70,
    child_population: 200,
    num_households: 100,
    num_schools: 1,
    num_health_workers: 2,
    handwash_station_coverage: 0.50,
    open_defecation_rate: 0.15,
    baseline_soil_contamination: 0.02,
    medicine_supply_reliability: 0.80,
    bhw_per_thousand: 3.0,
    mean_knowledge: 0.70,
    mean_attitude: 0.70,
    mean_practice: 0.65,
  }],
});

async function initWasm(): Promise<void> {
  const glueUrl = '/wasm/simulation.js';
  // @ts-ignore -- runtime path, not a TS module
  const glue = await import(/* webpackIgnore: true */ glueUrl);

  const wasmResponse = await fetch('/wasm/simulation_bg.wasm');
  const wasmBytes = await wasmResponse.arrayBuffer();
  await glue.default(wasmBytes);
  wasmExports = glue;
}

function getOrAllocBuffer(pool: ArrayBuffer | null, needed: number): ArrayBuffer {
  if (pool && pool.byteLength >= needed) return pool;
  return new ArrayBuffer(needed);
}

/** Pack render data from a single simulation instance */
function packSimData(
  simInstance: any,
  poolAgent: ArrayBuffer | null,
  poolEnv: ArrayBuffer | null,
  poolFacility: ArrayBuffer | null,
): {
  agentBuffer: ArrayBuffer;
  envBuffer: ArrayBuffer;
  facilityBuffer: ArrayBuffer;
  agentCount: number;
  envGridWidth: number;
  envGridHeight: number;
  envCellSize: number;
  statsJson: string;
  eventsJson: string;
} {
  const agentData: Float32Array = simInstance.get_agent_render_data();
  const agentCount = simInstance.get_agent_count();
  const agentNeeded = agentData.byteLength;
  const agentBuffer = getOrAllocBuffer(poolAgent, agentNeeded);
  new Float32Array(agentBuffer, 0, agentData.length).set(agentData);

  const envData: Float32Array = simInstance.get_env_render_data();
  const envNeeded = envData.byteLength;
  const envBuffer = getOrAllocBuffer(poolEnv, envNeeded);
  new Float32Array(envBuffer, 0, envData.length).set(envData);

  const facilityData: Float32Array = simInstance.get_facility_render_data();
  const facilityNeeded = facilityData.byteLength;
  const facilityBuffer = getOrAllocBuffer(poolFacility, facilityNeeded);
  new Float32Array(facilityBuffer, 0, facilityData.length).set(facilityData);

  const statsJson: string = simInstance.get_stats_json();
  const eventsJson: string = simInstance.drain_events_json();

  const envGridDims: Float32Array = simInstance.get_env_grid_dims();
  const envGridWidth: number = envGridDims[0] ?? 0;
  const envGridHeight: number = envGridDims[1] ?? 0;
  const envCellSize: number = envGridDims[2] ?? 10;

  return {
    agentBuffer,
    envBuffer,
    facilityBuffer,
    agentCount,
    envGridWidth,
    envGridHeight,
    envCellSize,
    statsJson,
    eventsJson,
  };
}

function packAndSendFrame() {
  if (!sim) return;

  // Pack primary simulation
  const d1 = packSimData(sim, agentPoolBuf, envPoolBuf, facilityPoolBuf);
  agentPoolBuf = null;
  envPoolBuf = null;
  facilityPoolBuf = null;

  const msg: SthWorkerMessage = {
    type: 'frame',
    agentBuffer: d1.agentBuffer,
    envBuffer: d1.envBuffer,
    facilityBuffer: d1.facilityBuffer,
    stats: d1.statsJson,
    events: d1.eventsJson,
    agentCount: d1.agentCount,
    envGridWidth: d1.envGridWidth,
    envGridHeight: d1.envGridHeight,
    envCellSize: d1.envCellSize,
  };

  const transfers: ArrayBuffer[] = [d1.agentBuffer, d1.envBuffer, d1.facilityBuffer];

  // Pack second simulation if comparison mode
  if (comparisonMode && sim2) {
    const d2 = packSimData(sim2, agentPoolBuf2, envPoolBuf2, facilityPoolBuf2);
    agentPoolBuf2 = null;
    envPoolBuf2 = null;
    facilityPoolBuf2 = null;

    msg.agentBuffer2 = d2.agentBuffer;
    msg.envBuffer2 = d2.envBuffer;
    msg.facilityBuffer2 = d2.facilityBuffer;
    msg.stats2 = d2.statsJson;
    msg.events2 = d2.eventsJson;
    msg.agentCount2 = d2.agentCount;
    msg.envGridWidth2 = d2.envGridWidth;
    msg.envGridHeight2 = d2.envGridHeight;
    msg.envCellSize2 = d2.envCellSize;

    transfers.push(d2.agentBuffer, d2.envBuffer, d2.facilityBuffer);
  }

  postMessage(msg, transfers as any);
}

function runLoop(ticksPerFrame: number) {
  if (!running || !sim) return;

  const start = performance.now();
  let ticksDone = 0;

  while (ticksDone < ticksPerFrame && performance.now() - start < TICK_BUDGET_MS) {
    sim.step();
    if (comparisonMode && sim2) {
      sim2.step();
    }
    ticksDone++;
  }

  packAndSendFrame();
  postMessage({ type: 'heartbeat' } as SthWorkerMessage);

  timeoutId = setTimeout(() => runLoop(ticksPerFrame), 0);
}

function stopLoop() {
  running = false;
  if (timeoutId !== null) {
    clearTimeout(timeoutId);
    timeoutId = null;
  }
}

/** Create the primary simulation */
function createSim1(seed?: number, config?: string): any {
  const { SthSimulation } = wasmExports;
  if (config) {
    return SthSimulation.with_config(config);
  } else if (seed !== undefined) {
    return SthSimulation.with_seed(BigInt(seed));
  } else {
    return new SthSimulation();
  }
}

/** Create the secondary (rural) simulation for comparison mode */
function createSim2(): any {
  const { SthSimulation } = wasmExports;
  return SthSimulation.with_config(RURAL_CONFIG);
}

/** Get the target simulation for an intervention command */
function getTargetSim(barangayId?: number): any {
  if (barangayId === 1 && comparisonMode && sim2) {
    return sim2;
  }
  return sim;
}

onmessage = async (e: MessageEvent<SthCommand>) => {
  const cmd = e.data;

  // Accept returned buffers for double-buffering
  if (cmd && typeof cmd === 'object' && 'returnBuffers' in cmd) {
    const ret = cmd as any;
    if (ret.agentBuffer instanceof ArrayBuffer) agentPoolBuf = ret.agentBuffer;
    if (ret.envBuffer instanceof ArrayBuffer) envPoolBuf = ret.envBuffer;
    if (ret.facilityBuffer instanceof ArrayBuffer) facilityPoolBuf = ret.facilityBuffer;
    // Comparison mode buffers
    if (ret.agentBuffer2 instanceof ArrayBuffer) agentPoolBuf2 = ret.agentBuffer2;
    if (ret.envBuffer2 instanceof ArrayBuffer) envPoolBuf2 = ret.envBuffer2;
    if (ret.facilityBuffer2 instanceof ArrayBuffer) facilityPoolBuf2 = ret.facilityBuffer2;
    return;
  }

  try {
    switch (cmd.type) {
      case 'init': {
        if (!wasmExports) {
          await initWasm();
        }
        comparisonMode = cmd.comparisonMode ?? false;

        if (comparisonMode) {
          sim = createSim1(cmd.seed, URBAN_CONFIG);
          sim2 = createSim2();
        } else {
          sim = createSim1(cmd.seed, cmd.config);
          sim2 = null;
        }

        const readyMsg: SthWorkerMessage = {
          type: 'ready',
          seed: Number(sim.get_seed()),
          worldWidth: sim.get_world_width(),
          worldHeight: sim.get_world_height(),
        };
        postMessage(readyMsg);
        break;
      }

      case 'step': {
        if (!sim) return;
        running = true;
        runLoop(cmd.count);
        break;
      }

      case 'pause': {
        stopLoop();
        break;
      }

      case 'resume': {
        if (!sim) return;
        running = true;
        runLoop(1);
        break;
      }

      case 'reset': {
        stopLoop();
        if (wasmExports) {
          comparisonMode = cmd.comparisonMode ?? comparisonMode;

          if (comparisonMode) {
            sim = createSim1(cmd.seed, URBAN_CONFIG);
            sim2 = createSim2();
          } else {
            sim = createSim1(cmd.seed, cmd.config);
            sim2 = null;
          }

          packAndSendFrame();
        }
        const resetMsg: SthWorkerMessage = {
          type: 'ready',
          seed: sim ? Number(sim.get_seed()) : 0,
          worldWidth: sim ? sim.get_world_width() : 800,
          worldHeight: sim ? sim.get_world_height() : 600,
        };
        postMessage(resetMsg);
        break;
      }

      case 'toggleComparisonMode': {
        if (!wasmExports) break;
        const wasRunning = running;
        stopLoop();

        comparisonMode = cmd.enabled;
        if (comparisonMode) {
          sim = createSim1(undefined, URBAN_CONFIG);
          sim2 = createSim2();
        } else {
          sim = createSim1();
          sim2 = null;
        }

        packAndSendFrame();

        const toggleMsg: SthWorkerMessage = {
          type: 'ready',
          seed: sim ? Number(sim.get_seed()) : 0,
          worldWidth: sim ? sim.get_world_width() : 800,
          worldHeight: sim ? sim.get_world_height() : 600,
        };
        postMessage(toggleMsg);

        // Resume if was running
        if (wasRunning) {
          running = true;
          runLoop(1);
        }
        break;
      }

      // -- Intervention commands (with barangayId targeting) --

      case 'launchMDA': {
        const target = getTargetSim(cmd.barangayId);
        if (target) {
          target.launch_mda(cmd.schoolId, cmd.drug);
        }
        break;
      }

      case 'buildLatrine': {
        const target = getTargetSim(cmd.barangayId);
        if (target) {
          target.build_latrine(cmd.x, cmd.y);
        }
        break;
      }

      case 'buildWaterPump': {
        const target = getTargetSim(cmd.barangayId);
        if (target) {
          target.build_water_pump(cmd.x, cmd.y);
        }
        break;
      }

      case 'buildHandwashStation': {
        const target = getTargetSim(cmd.barangayId);
        if (target) {
          target.build_handwash_station(cmd.x, cmd.y);
        }
        break;
      }

      case 'launchEducation': {
        const target = getTargetSim(cmd.barangayId);
        if (target) {
          target.launch_education(cmd.method, cmd.schoolId);
        }
        break;
      }

      case 'launchBhwVisits': {
        const target = getTargetSim(cmd.barangayId);
        if (target) {
          target.launch_bhw_visits(cmd.coverage);
        }
        break;
      }

      case 'increaseBudget': {
        const target = getTargetSim(cmd.barangayId);
        if (target) {
          target.increase_budget(cmd.multiplier);
        }
        break;
      }

      case 'selectAgent': {
        selectedAgentIdx = cmd.index;
        if (sim && cmd.index !== null) {
          try {
            const detailJson: string = sim.get_agent_detail_json(cmd.index);
            const detailMsg: SthWorkerMessage = {
              type: 'agentDetail',
              detail: detailJson,
            };
            postMessage(detailMsg);
          } catch {
            // Agent may not exist at this index
          }
        }
        break;
      }

      case 'returnBuffer': {
        // Individual buffer return (handled by the returnBuffers path above)
        break;
      }
    }
  } catch (err: any) {
    postMessage({
      type: 'error',
      message: err?.message || String(err),
    } as SthWorkerMessage);
  }
};
