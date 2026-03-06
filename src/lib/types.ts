export const FLOATS_PER_CREATURE = 12;
// [x, y, rotation, size, energy_norm, r, g, b, vx, vy, age_norm, species_id]

export interface SimStats {
  tick: number;
  creatureCount: number;
  foodCount: number;
  generationMax: number;
  totalBirths: number;
  totalDeaths: number;
  nanDeaths: number;
  energyDriftPct: number;
  profileJson: string;
}

export interface TickProfile {
  spatial_hash_us: number;
  physics_us: number;
  energy_us: number;
  reproduction_us: number;
  food_us: number;
  cleanup_us: number;
  render_pack_us: number;
  total_us: number;
  creature_count: number;
  food_count: number;
}

export type SimCommand =
  | { type: 'init'; seed?: number }
  | { type: 'step'; count: number }
  | { type: 'pause' }
  | { type: 'resume' }
  | { type: 'reset'; seed?: number };

export type WorkerMessage =
  | { type: 'ready' }
  | { type: 'frame'; creatureBuffer: ArrayBuffer; foodBuffer: ArrayBuffer; stats: SimStats; worldWidth: number; worldHeight: number }
  | { type: 'error'; message: string }
  | { type: 'heartbeat' };
