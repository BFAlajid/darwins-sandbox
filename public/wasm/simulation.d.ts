/* tslint:disable */
/* eslint-disable */

export class SthSimulation {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Build a handwash station at the given world position.
     */
    build_handwash_station(x: number, y: number): void;
    /**
     * Build a latrine at the given world position.
     */
    build_latrine(x: number, y: number): void;
    /**
     * Build a water pump at the given world position.
     */
    build_water_pump(x: number, y: number): void;
    /**
     * Get new events since last call as JSON (incremental).
     */
    drain_events_json(): string;
    /**
     * Get the number of agents in the simulation.
     */
    get_agent_count(): number;
    /**
     * Get detailed information about a specific agent by index.
     */
    get_agent_detail_json(index: number): string;
    /**
     * Get agent render data as a flat f32 array (16 floats per agent).
     */
    get_agent_render_data(): Float32Array;
    /**
     * Get the remaining budget.
     */
    get_budget_remaining(): number;
    /**
     * Get the total budget spent.
     */
    get_budget_spent(): number;
    /**
     * Get the current day of simulation (0-indexed).
     */
    get_day(): number;
    /**
     * Get environment grid dimensions: [width, height, cell_size].
     */
    get_env_grid_dims(): Float32Array;
    /**
     * Get environment contamination grid as a flat f32 array.
     */
    get_env_render_data(): Float32Array;
    /**
     * Get facility positions as a flat f32 array (4 floats per facility).
     */
    get_facility_render_data(): Float32Array;
    /**
     * Get the current month of simulation (0-indexed, 30-day months).
     */
    get_month(): number;
    /**
     * Get the random seed used for this simulation.
     */
    get_seed(): bigint;
    /**
     * Get simulation statistics as a JSON string.
     */
    get_stats_json(): string;
    /**
     * Get the current tick number.
     */
    get_tick(): bigint;
    /**
     * Get the world height in pixels.
     */
    get_world_height(): number;
    /**
     * Get the world width in pixels.
     */
    get_world_width(): number;
    /**
     * Increase the monthly budget increment by a multiplier.
     */
    increase_budget(multiplier: number): void;
    /**
     * Launch BHW house-to-house visits.
     * coverage: fraction of households to visit (0.0-1.0).
     */
    launch_bhw_visits(coverage: number): void;
    /**
     * Launch a health education campaign.
     * method: 0=Cartoon, 1=BoardGame, 2=TeacherLed, 3=ParentMeeting.
     * school_id: -1 for all schools, 0+ for specific school.
     */
    launch_education(method: number, school_id: number): void;
    /**
     * Launch Mass Drug Administration.
     * school_id: -1 for all schools, 0+ for specific school.
     * drug: 0 = Albendazole, 1 = Mebendazole.
     */
    launch_mda(school_id: number, drug: number): void;
    /**
     * Create a new simulation with default configuration.
     */
    constructor();
    /**
     * Advance the simulation by one tick (1 hour of simulated time).
     */
    step(): void;
    /**
     * Create a new simulation from a JSON configuration string.
     */
    static with_config(config_json: string): SthSimulation;
    /**
     * Create a new simulation with a specific random seed.
     */
    static with_seed(seed: bigint): SthSimulation;
}

export function init_panic_hook(): void;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_sthsimulation_free: (a: number, b: number) => void;
    readonly sthsimulation_build_handwash_station: (a: number, b: number, c: number) => void;
    readonly sthsimulation_build_latrine: (a: number, b: number, c: number) => void;
    readonly sthsimulation_build_water_pump: (a: number, b: number, c: number) => void;
    readonly sthsimulation_drain_events_json: (a: number) => [number, number];
    readonly sthsimulation_get_agent_count: (a: number) => number;
    readonly sthsimulation_get_agent_detail_json: (a: number, b: number) => [number, number];
    readonly sthsimulation_get_agent_render_data: (a: number) => [number, number];
    readonly sthsimulation_get_budget_remaining: (a: number) => number;
    readonly sthsimulation_get_budget_spent: (a: number) => number;
    readonly sthsimulation_get_day: (a: number) => number;
    readonly sthsimulation_get_env_grid_dims: (a: number) => [number, number];
    readonly sthsimulation_get_env_render_data: (a: number) => [number, number];
    readonly sthsimulation_get_facility_render_data: (a: number) => [number, number];
    readonly sthsimulation_get_month: (a: number) => number;
    readonly sthsimulation_get_seed: (a: number) => bigint;
    readonly sthsimulation_get_stats_json: (a: number) => [number, number];
    readonly sthsimulation_get_tick: (a: number) => bigint;
    readonly sthsimulation_get_world_height: (a: number) => number;
    readonly sthsimulation_get_world_width: (a: number) => number;
    readonly sthsimulation_increase_budget: (a: number, b: number) => void;
    readonly sthsimulation_launch_bhw_visits: (a: number, b: number) => void;
    readonly sthsimulation_launch_education: (a: number, b: number, c: number) => void;
    readonly sthsimulation_launch_mda: (a: number, b: number, c: number) => void;
    readonly sthsimulation_new: () => [number, number, number];
    readonly sthsimulation_step: (a: number) => void;
    readonly sthsimulation_with_config: (a: number, b: number) => [number, number, number];
    readonly sthsimulation_with_seed: (a: bigint) => [number, number, number];
    readonly init_panic_hook: () => void;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
