/* tslint:disable */
/* eslint-disable */

export class Simulation {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Get current creature count.
     */
    get_creature_count(): number;
    /**
     * Get creature render data as a copied Vec<f32>.
     * Simpler than pointer-based access — no memory management needed on JS side.
     */
    get_creature_render_data(): Float32Array;
    /**
     * Get energy drift percentage.
     */
    get_energy_drift_pct(): number;
    /**
     * Get current food count.
     */
    get_food_count(): number;
    /**
     * Get food render data as flat f32 array: [x, y, energy, ...]
     */
    get_food_render_data(): Float32Array;
    /**
     * Get maximum generation observed.
     */
    get_generation_max(): number;
    /**
     * Get NaN death count (should always be 0 in normal operation).
     */
    get_nan_deaths(): number;
    /**
     * Get tick profile as JSON string.
     */
    get_profile_json(): string;
    /**
     * Get length of render data buffer (number of f32 values).
     */
    get_render_data_len(): number;
    /**
     * Get pointer to creature render data (flat f32 buffer).
     * MUST re-acquire Float32Array view after every step() call.
     */
    get_render_data_ptr(): number;
    /**
     * Get current tick number.
     */
    get_tick(): bigint;
    /**
     * Get total births since start.
     */
    get_total_births(): bigint;
    /**
     * Get total deaths since start.
     */
    get_total_deaths(): bigint;
    /**
     * Get world height.
     */
    get_world_height(): number;
    /**
     * Get world width.
     */
    get_world_width(): number;
    /**
     * Create a new simulation with default config.
     */
    constructor();
    /**
     * Advance the simulation by one tick.
     */
    step(): void;
    /**
     * Create a new simulation from a JSON config string.
     */
    static with_config(config_json: string): Simulation;
    /**
     * Create a new simulation with a specific seed.
     */
    static with_seed(seed: bigint): Simulation;
}

export function init_panic_hook(): void;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_simulation_free: (a: number, b: number) => void;
    readonly simulation_get_creature_count: (a: number) => number;
    readonly simulation_get_creature_render_data: (a: number) => [number, number];
    readonly simulation_get_energy_drift_pct: (a: number) => number;
    readonly simulation_get_food_count: (a: number) => number;
    readonly simulation_get_food_render_data: (a: number) => [number, number];
    readonly simulation_get_generation_max: (a: number) => number;
    readonly simulation_get_nan_deaths: (a: number) => number;
    readonly simulation_get_profile_json: (a: number) => [number, number];
    readonly simulation_get_render_data_len: (a: number) => number;
    readonly simulation_get_render_data_ptr: (a: number) => number;
    readonly simulation_get_tick: (a: number) => bigint;
    readonly simulation_get_total_births: (a: number) => bigint;
    readonly simulation_get_total_deaths: (a: number) => bigint;
    readonly simulation_get_world_height: (a: number) => number;
    readonly simulation_get_world_width: (a: number) => number;
    readonly simulation_new: () => [number, number, number];
    readonly simulation_step: (a: number) => void;
    readonly simulation_with_config: (a: number, b: number) => [number, number, number];
    readonly simulation_with_seed: (a: bigint) => [number, number, number];
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
