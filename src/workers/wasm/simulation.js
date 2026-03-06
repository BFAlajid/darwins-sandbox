/* @ts-self-types="./simulation.d.ts" */

export class Simulation {
    static __wrap(ptr) {
        ptr = ptr >>> 0;
        const obj = Object.create(Simulation.prototype);
        obj.__wbg_ptr = ptr;
        SimulationFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        SimulationFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_simulation_free(ptr, 0);
    }
    /**
     * Get current creature count.
     * @returns {number}
     */
    get_creature_count() {
        const ret = wasm.simulation_get_creature_count(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Get creature render data as a copied Vec<f32>.
     * Simpler than pointer-based access — no memory management needed on JS side.
     * @returns {Float32Array}
     */
    get_creature_render_data() {
        const ret = wasm.simulation_get_creature_render_data(this.__wbg_ptr);
        var v1 = getArrayF32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * Get energy drift percentage.
     * @returns {number}
     */
    get_energy_drift_pct() {
        const ret = wasm.simulation_get_energy_drift_pct(this.__wbg_ptr);
        return ret;
    }
    /**
     * Get current food count.
     * @returns {number}
     */
    get_food_count() {
        const ret = wasm.simulation_get_food_count(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Get food render data as flat f32 array: [x, y, energy, ...]
     * @returns {Float32Array}
     */
    get_food_render_data() {
        const ret = wasm.simulation_get_food_render_data(this.__wbg_ptr);
        var v1 = getArrayF32FromWasm0(ret[0], ret[1]).slice();
        wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
        return v1;
    }
    /**
     * Get maximum generation observed.
     * @returns {number}
     */
    get_generation_max() {
        const ret = wasm.simulation_get_generation_max(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Get NaN death count (should always be 0 in normal operation).
     * @returns {number}
     */
    get_nan_deaths() {
        const ret = wasm.simulation_get_nan_deaths(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Get tick profile as JSON string.
     * @returns {string}
     */
    get_profile_json() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.simulation_get_profile_json(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * Get length of render data buffer (number of f32 values).
     * @returns {number}
     */
    get_render_data_len() {
        const ret = wasm.simulation_get_render_data_len(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Get pointer to creature render data (flat f32 buffer).
     * MUST re-acquire Float32Array view after every step() call.
     * @returns {number}
     */
    get_render_data_ptr() {
        const ret = wasm.simulation_get_render_data_ptr(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Get current tick number.
     * @returns {bigint}
     */
    get_tick() {
        const ret = wasm.simulation_get_tick(this.__wbg_ptr);
        return BigInt.asUintN(64, ret);
    }
    /**
     * Get total births since start.
     * @returns {bigint}
     */
    get_total_births() {
        const ret = wasm.simulation_get_total_births(this.__wbg_ptr);
        return BigInt.asUintN(64, ret);
    }
    /**
     * Get total deaths since start.
     * @returns {bigint}
     */
    get_total_deaths() {
        const ret = wasm.simulation_get_total_deaths(this.__wbg_ptr);
        return BigInt.asUintN(64, ret);
    }
    /**
     * Get world height.
     * @returns {number}
     */
    get_world_height() {
        const ret = wasm.simulation_get_world_height(this.__wbg_ptr);
        return ret;
    }
    /**
     * Get world width.
     * @returns {number}
     */
    get_world_width() {
        const ret = wasm.simulation_get_world_width(this.__wbg_ptr);
        return ret;
    }
    /**
     * Create a new simulation with default config.
     */
    constructor() {
        const ret = wasm.simulation_new();
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        this.__wbg_ptr = ret[0] >>> 0;
        SimulationFinalization.register(this, this.__wbg_ptr, this);
        return this;
    }
    /**
     * Advance the simulation by one tick.
     */
    step() {
        wasm.simulation_step(this.__wbg_ptr);
    }
    /**
     * Create a new simulation from a JSON config string.
     * @param {string} config_json
     * @returns {Simulation}
     */
    static with_config(config_json) {
        const ptr0 = passStringToWasm0(config_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
        const len0 = WASM_VECTOR_LEN;
        const ret = wasm.simulation_with_config(ptr0, len0);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return Simulation.__wrap(ret[0]);
    }
    /**
     * Create a new simulation with a specific seed.
     * @param {bigint} seed
     * @returns {Simulation}
     */
    static with_seed(seed) {
        const ret = wasm.simulation_with_seed(seed);
        if (ret[2]) {
            throw takeFromExternrefTable0(ret[1]);
        }
        return Simulation.__wrap(ret[0]);
    }
}
if (Symbol.dispose) Simulation.prototype[Symbol.dispose] = Simulation.prototype.free;

export function init_panic_hook() {
    wasm.init_panic_hook();
}

function __wbg_get_imports() {
    const import0 = {
        __proto__: null,
        __wbg___wbindgen_throw_6ddd609b62940d55: function(arg0, arg1) {
            throw new Error(getStringFromWasm0(arg0, arg1));
        },
        __wbg_error_a6fa202b58aa1cd3: function(arg0, arg1) {
            let deferred0_0;
            let deferred0_1;
            try {
                deferred0_0 = arg0;
                deferred0_1 = arg1;
                console.error(getStringFromWasm0(arg0, arg1));
            } finally {
                wasm.__wbindgen_free(deferred0_0, deferred0_1, 1);
            }
        },
        __wbg_new_227d7c05414eb861: function() {
            const ret = new Error();
            return ret;
        },
        __wbg_now_16f0c993d5dd6c27: function() {
            const ret = Date.now();
            return ret;
        },
        __wbg_stack_3b0d974bbf31e44f: function(arg0, arg1) {
            const ret = arg1.stack;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbindgen_cast_0000000000000001: function(arg0, arg1) {
            // Cast intrinsic for `Ref(String) -> Externref`.
            const ret = getStringFromWasm0(arg0, arg1);
            return ret;
        },
        __wbindgen_init_externref_table: function() {
            const table = wasm.__wbindgen_externrefs;
            const offset = table.grow(4);
            table.set(0, undefined);
            table.set(offset + 0, undefined);
            table.set(offset + 1, null);
            table.set(offset + 2, true);
            table.set(offset + 3, false);
        },
    };
    return {
        __proto__: null,
        "./simulation_bg.js": import0,
    };
}

const SimulationFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_simulation_free(ptr >>> 0, 1));

function getArrayF32FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getFloat32ArrayMemory0().subarray(ptr / 4, ptr / 4 + len);
}

let cachedDataViewMemory0 = null;
function getDataViewMemory0() {
    if (cachedDataViewMemory0 === null || cachedDataViewMemory0.buffer.detached === true || (cachedDataViewMemory0.buffer.detached === undefined && cachedDataViewMemory0.buffer !== wasm.memory.buffer)) {
        cachedDataViewMemory0 = new DataView(wasm.memory.buffer);
    }
    return cachedDataViewMemory0;
}

let cachedFloat32ArrayMemory0 = null;
function getFloat32ArrayMemory0() {
    if (cachedFloat32ArrayMemory0 === null || cachedFloat32ArrayMemory0.byteLength === 0) {
        cachedFloat32ArrayMemory0 = new Float32Array(wasm.memory.buffer);
    }
    return cachedFloat32ArrayMemory0;
}

function getStringFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return decodeText(ptr, len);
}

let cachedUint8ArrayMemory0 = null;
function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.byteLength === 0) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

function passStringToWasm0(arg, malloc, realloc) {
    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length, 1) >>> 0;
        getUint8ArrayMemory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len, 1) >>> 0;

    const mem = getUint8ArrayMemory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }
    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3, 1) >>> 0;
        const view = getUint8ArrayMemory0().subarray(ptr + offset, ptr + len);
        const ret = cachedTextEncoder.encodeInto(arg, view);

        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

function takeFromExternrefTable0(idx) {
    const value = wasm.__wbindgen_externrefs.get(idx);
    wasm.__externref_table_dealloc(idx);
    return value;
}

let cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
cachedTextDecoder.decode();
const MAX_SAFARI_DECODE_BYTES = 2146435072;
let numBytesDecoded = 0;
function decodeText(ptr, len) {
    numBytesDecoded += len;
    if (numBytesDecoded >= MAX_SAFARI_DECODE_BYTES) {
        cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
        cachedTextDecoder.decode();
        numBytesDecoded = len;
    }
    return cachedTextDecoder.decode(getUint8ArrayMemory0().subarray(ptr, ptr + len));
}

const cachedTextEncoder = new TextEncoder();

if (!('encodeInto' in cachedTextEncoder)) {
    cachedTextEncoder.encodeInto = function (arg, view) {
        const buf = cachedTextEncoder.encode(arg);
        view.set(buf);
        return {
            read: arg.length,
            written: buf.length
        };
    };
}

let WASM_VECTOR_LEN = 0;

let wasmModule, wasm;
function __wbg_finalize_init(instance, module) {
    wasm = instance.exports;
    wasmModule = module;
    cachedDataViewMemory0 = null;
    cachedFloat32ArrayMemory0 = null;
    cachedUint8ArrayMemory0 = null;
    wasm.__wbindgen_start();
    return wasm;
}

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);
            } catch (e) {
                const validResponse = module.ok && expectedResponseType(module.type);

                if (validResponse && module.headers.get('Content-Type') !== 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else { throw e; }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);
    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };
        } else {
            return instance;
        }
    }

    function expectedResponseType(type) {
        switch (type) {
            case 'basic': case 'cors': case 'default': return true;
        }
        return false;
    }
}

function initSync(module) {
    if (wasm !== undefined) return wasm;


    if (module !== undefined) {
        if (Object.getPrototypeOf(module) === Object.prototype) {
            ({module} = module)
        } else {
            console.warn('using deprecated parameters for `initSync()`; pass a single object instead')
        }
    }

    const imports = __wbg_get_imports();
    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }
    const instance = new WebAssembly.Instance(module, imports);
    return __wbg_finalize_init(instance, module);
}

async function __wbg_init(module_or_path) {
    if (wasm !== undefined) return wasm;


    if (module_or_path !== undefined) {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({module_or_path} = module_or_path)
        } else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead')
        }
    }

    if (module_or_path === undefined) {
        module_or_path = new URL('simulation_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports();

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module);
}

export { initSync, __wbg_init as default };
