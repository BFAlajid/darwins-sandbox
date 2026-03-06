# Darwin's Sandbox -- Security Audit

**Date:** 2026-03-06
**Auditor Role:** Security Reviewer
**Scope:** Full architecture threat model -- pre-implementation research audit
**Status:** Pre-merge gate. No code exists yet; this document establishes security requirements that implementation MUST satisfy.

---

## Table of Contents

1. [Threat Model](#1-threat-model)
2. [WASM Security](#2-wasm-security)
3. [Web Worker Security](#3-web-worker-security)
4. [User Input Validation](#4-user-input-validation)
5. [Save/Load Binary Security](#5-saveload-binary-security)
6. [IndexedDB Security](#6-indexeddb-security)
7. [Canvas/Rendering Security](#7-canvasrendering-security)
8. [Supply Chain Security](#8-supply-chain-security)
9. [Cross-Origin and Network Security](#9-cross-origin-and-network-security)
10. [Denial of Service (Self-Inflicted)](#10-denial-of-service-self-inflicted)
11. [Privacy](#11-privacy)
12. [Summary Risk Matrix](#12-summary-risk-matrix)
13. [Required Security Tests](#13-required-security-tests)
14. [Residual Risk](#14-residual-risk)

---

## 1. Threat Model

### 1.1 Application Profile

Darwin's Sandbox is a **client-side-only** browser application. There is no backend server, no user accounts, no database, and no authentication. All computation happens in the user's browser. This dramatically reduces the attack surface compared to a typical web application.

### 1.2 Trust Boundaries

```
+-----------------------------------------------------------------------+
|  BROWSER SANDBOX (Origin: darwins-sandbox.vercel.app)                 |
|                                                                       |
|  +-----------------------------+    +------------------------------+  |
|  |  MAIN THREAD                |    |  WEB WORKER THREAD           |  |
|  |                             |    |                              |  |
|  |  React UI (trusted)        |    |  +------------------------+  |  |
|  |  Zustand store (trusted)   |<-->|  |  WASM LINEAR MEMORY    |  |  |
|  |  Canvas renderer (trusted) | msg|  |  (sandboxed)           |  |  |
|  |  URL params (UNTRUSTED)    |    |  |  Rust simulation       |  |  |
|  |  File upload (UNTRUSTED)   |    |  |  dlmalloc heap         |  |  |
|  |                             |    |  +------------------------+  |  |
|  +-----------------------------+    |                              |  |
|                                     |  IndexedDB (same-origin)    |  |
|                                     +------------------------------+  |
|                                                                       |
|  UNTRUSTED INPUTS:                                                    |
|  - URL query parameters (seed, config)                                |
|  - Uploaded binary save files                                         |
|  - Modified slider values (via DevTools)                              |
+-----------------------------------------------------------------------+
|                                                                       |
|  EXTERNAL (outside browser sandbox):                                  |
|  - npm/crate supply chain                                             |
|  - CDN/Vercel serving static assets                                   |
|  - User's filesystem (save file source)                               |
+-----------------------------------------------------------------------+
```

### 1.3 Threat Actors

| Actor | Motivation | Capability |
|-------|-----------|------------|
| Curious user | Push limits, crash sim for fun | DevTools, URL manipulation, crafted save files |
| Malicious link sharer | Crash victim's browser tab, phishing | Crafted URL with extreme parameters |
| Supply chain attacker | Inject malicious code into dependency | Compromised npm/crate package |
| XSS attacker (if future features add UGC) | Session hijack, data theft | Injected scripts via unsanitized input |

### 1.4 Assets at Risk

| Asset | Value | Notes |
|-------|-------|-------|
| User's browser tab stability | Medium | A crash loses unsaved simulation state |
| User's browser performance | Medium | Runaway simulation can freeze the tab |
| User's storage quota | Low | IndexedDB abuse can consume disk |
| User's privacy | Low | Canvas fingerprinting, no PII collected |
| Application integrity | Medium | Malformed saves could corrupt simulation state |

### 1.5 Out of Scope

- Server-side attacks (no server)
- Authentication/authorization (no accounts)
- Database injection (no database)
- Network interception of user data (no sensitive data transmitted)

---

## 2. WASM Security

### F-2.1: WASM Linear Memory Sandbox Integrity

**Risk Level: LOW**

**Analysis:** WebAssembly operates within a linear memory sandbox enforced by the browser engine (V8, SpiderMonkey, JavaScriptCore). WASM code:

- CANNOT access JavaScript objects, the DOM, cookies, localStorage, or any browser API directly.
- CANNOT read or write memory outside its own linear memory ArrayBuffer.
- CANNOT make network requests, access the filesystem, or invoke system calls.
- CAN only call functions explicitly imported via the WASM import object (which wasm-bindgen generates).

The sandbox is enforced at the engine level with bounds-checked memory access. Every load/store instruction is validated against the linear memory bounds. This is a hardware-enforced boundary on modern engines (using guard pages / virtual memory tricks), not a software check on every access.

**wasm-bindgen boundary:** The generated JS glue code IS the attack surface. wasm-bindgen creates JS functions that the WASM module can call. These are limited to what the developer exports. For Darwin's Sandbox, the exports should be minimal: `new()`, `step()`, `get_render_data_ptr()`, `get_render_data_len()`, `update_config()`, `serialize_state()`, `deserialize_state()`.

**Mitigation:**
- Keep the wasm-bindgen export surface minimal. Do not export arbitrary memory access functions.
- Audit all `#[wasm_bindgen]` exports to ensure none provide unintended capabilities.
- Do not import `js_sys` functions that provide access to `eval`, `Function`, or dynamic code execution.

### F-2.2: WASM Panic Behavior

**Risk Level: LOW**

**Analysis:** When Rust code panics in WASM:

1. With `console_error_panic_hook` (which the architecture specifies): the panic message and backtrace are logged to the browser console.
2. The panic unwinds (or aborts, depending on `panic = "abort"` in Cargo.toml profile).
3. With `panic = "abort"`: the WASM instance is terminated. The linear memory becomes inaccessible. The Worker continues running but the WASM module is dead.
4. With `panic = "unwind"`: the stack unwinds through wasm-bindgen, which converts it to a JS exception thrown from the WASM call site.

**Can a panic corrupt memory?** No. A panic does NOT corrupt WASM linear memory in a way that affects the host. The linear memory sandbox remains intact. After a panic-abort, the memory is simply abandoned. After a panic-unwind, the WASM instance may be in an inconsistent internal state (Rust destructors may not have run for all values), but this is contained within the WASM sandbox.

**Can a panic be weaponized?** Only as a denial-of-service (crashing the simulation). A crafted input that triggers a panic kills the WASM instance but cannot escape the sandbox.

**Mitigation:**
- Use `panic = "abort"` in the release profile to reduce WASM binary size and eliminate unwind tables.
- The Worker's `onerror` handler should catch the termination and report a clean error state to the main thread.
- After a WASM panic, the Worker should be capable of re-initializing a fresh WASM instance without requiring a page reload.

### F-2.3: WASM Sandbox Escape

**Risk Level: LOW**

**Analysis:** There are no known, unpatched WASM sandbox escapes in modern browsers as of early 2026. The WASM sandbox is a simpler, more constrained model than the JavaScript sandbox and has had fewer escape vulnerabilities historically. Browser vendors treat WASM sandbox escapes as critical-severity bugs and patch them rapidly.

Theoretical escape vectors:
- JIT compiler bugs in the WASM engine (same class as JS JIT bugs)
- Implementation bugs in the linear memory bounds checking

These are browser engine vulnerabilities, not application-level vulnerabilities. Darwin's Sandbox cannot mitigate or exacerbate them.

**Mitigation:** None required at the application level. Users should keep browsers updated.

### F-2.4: Timing Side-Channel Attacks (Spectre/Meltdown)

**Risk Level: LOW (for this application)**

**Analysis:** WASM code CAN be used as a vector for Spectre-class timing attacks because it provides:
- High-resolution timers (via `performance.now()` if imported, or self-constructed timers using SharedArrayBuffer)
- Fine-grained memory access patterns
- Branch prediction exploitation through crafted code

However, browsers have deployed multiple mitigations:
- `performance.now()` resolution reduced to 5 microseconds (with jitter) in cross-origin contexts
- SharedArrayBuffer requires COOP/COEP headers (cross-origin isolation)
- Site isolation (process-per-origin) prevents cross-origin data from being in the same address space
- V8 has deployed Spectre-resistant code generation for WASM

**Relevance to Darwin's Sandbox:** This application does not handle sensitive cross-origin data. There is nothing valuable to steal via a side-channel attack. The user's own simulation data is not secret. This attack class is irrelevant to the threat model.

**Mitigation:** None required. Do not import high-resolution timers into the WASM module unless needed. Do not enable SharedArrayBuffer unless specifically required (see Section 9).

### F-2.5: wasm-bindgen Generated Code

**Risk Level: LOW**

**Analysis:** wasm-bindgen is the most widely used Rust-to-WASM binding generator, maintained by the Rust/wasm working group. No CVEs have been filed against wasm-bindgen itself as of my knowledge cutoff. The generated JS glue code is mechanical and deterministic -- it marshals types between JS and WASM linear memory.

Known considerations:
- String passing allocates in WASM linear memory and copies. No buffer overflow risk because Rust's allocator handles sizing.
- Returning `Vec<u8>` or slices creates JS typed array views into WASM memory. These views are invalidated when WASM memory grows (the architecture document correctly notes this).
- The `--target web` output is an ES module with no Node.js dependencies, reducing supply chain surface.

**Mitigation:**
- Pin wasm-bindgen to a specific version in Cargo.toml.
- Run `cargo audit` in CI to catch any future advisories.
- Review the generated JS glue file after each wasm-bindgen version upgrade.

---

## 3. Web Worker Security

### F-3.1: Worker Isolation Boundary

**Risk Level: LOW**

**Analysis:** Web Workers operate in a separate global scope (`DedicatedWorkerGlobalScope`). A Worker:

- CANNOT access the DOM (`document`, `window` are undefined)
- CANNOT access `localStorage` or `sessionStorage`
- CANNOT access cookies
- CANNOT access the parent page's JavaScript variables or objects
- CAN access `IndexedDB`, `fetch`, `WebSocket`, `importScripts`, `crypto`, `performance`
- CAN only communicate with the main thread via `postMessage` / `onmessage`

The communication channel (structured clone algorithm + transferable objects) is the trust boundary. Data is either deep-cloned or ownership-transferred. There is no shared memory unless SharedArrayBuffer is explicitly used.

**Mitigation:** No action required. The Worker isolation model is well-suited to this architecture.

### F-3.2: Runaway Worker (Infinite Loop / WASM Hang)

**Risk Level: MEDIUM**

**Analysis:** If the WASM simulation enters an infinite loop (e.g., due to a bug in the physics or spatial hash code, or a malicious config that causes step() to never return), the Worker thread will hang. Effects:

- The Worker thread becomes unresponsive to postMessage commands (including stop/reset).
- The main thread is NOT affected -- UI remains responsive.
- The browser tab's memory usage continues to grow if the loop allocates.
- The user's only recourse is to close the tab or use the browser's task manager.
- There is NO JavaScript API to forcibly terminate a Worker's currently executing synchronous code from the outside. `Worker.terminate()` is available BUT it may not interrupt a synchronous WASM call on all browsers -- behavior is implementation-defined.

**Mitigation:**
- **Heartbeat protocol:** The Worker should send periodic heartbeat messages during long operations. The main thread should monitor for heartbeat timeout (e.g., 5 seconds without a heartbeat). If timeout occurs, call `Worker.terminate()` and spawn a new Worker.
- **Tick budget in WASM:** The `step()` function should accept a maximum tick count parameter. The fixed-timestep accumulator already caps at `TICK_RATE * 5` -- verify this is enforced.
- **Watchdog timer on main thread:**
  ```
  // Pseudocode
  let lastHeartbeat = Date.now();
  worker.onmessage = (e) => { lastHeartbeat = Date.now(); ... };
  setInterval(() => {
    if (Date.now() - lastHeartbeat > 5000) {
      worker.terminate();
      showError("Simulation became unresponsive. Restarting...");
      spawnNewWorker();
    }
  }, 1000);
  ```

### F-3.3: Malformed Messages from Worker

**Risk Level: LOW**

**Analysis:** The Worker sends two types of data to the main thread:
1. Transferable ArrayBuffer containing render data (Float32Array of creature positions/colors)
2. JSON stats object

If the WASM module has a bug and produces garbage render data, the main thread will attempt to render nonsensical values on the Canvas. This cannot cause a security vulnerability -- Canvas 2D operations with out-of-range coordinates simply draw off-screen or produce visual artifacts. NaN/Infinity values in Canvas coordinates are handled gracefully by all modern browsers (the draw call becomes a no-op or clips).

If the stats JSON contains unexpected types or values, Zustand will store them and React will render them. Since no stats values are used in `dangerouslySetInnerHTML` or injected into the DOM as raw HTML, there is no XSS risk.

**Mitigation:**
- Validate the ArrayBuffer length before creating a Float32Array view: `if (buffer.byteLength % (FLOATS_PER_CREATURE * 4) !== 0) { reject; }`
- Validate stats values are finite numbers before storing in Zustand.
- Type-check message shape: `if (msg.type !== 'frame' && msg.type !== 'stats') { ignore; }`

### F-3.4: Worker Spawning and Origin

**Risk Level: LOW**

**Analysis:** Workers can only load scripts from the same origin. The Worker script (`simulation.worker.ts`) is bundled by webpack and served from the same origin as the application. The WASM binary is loaded from `/public/wasm/` on the same origin. There is no cross-origin Worker loading.

**Mitigation:** None required. Ensure the Worker script and WASM binary are served from the same origin. Do not use `importScripts()` with external URLs.

---

## 4. User Input Validation

### F-4.1: Extreme Configuration Values via DevTools

**Risk Level: HIGH**

**Analysis:** The architecture specifies user-configurable parameters via UI sliders. A user can bypass slider min/max constraints using browser DevTools to send arbitrary values through Comlink to the Worker. Examples:

| Parameter | Dangerous Value | Impact |
|-----------|----------------|--------|
| `world_width` | 999999999 | Spatial hash grid allocation: `(999999999/cell_size)^2` cells = OOM crash |
| `world_height` | 999999999 | Same as above |
| `initial_population` | 10000000 | SlotMap allocation + neural nets = OOM |
| `hidden_neurons` | 999999 | Genome size explodes: `12 * 999999 + 999999 * 3` = millions of f32s per creature |
| `mutation_rate` | -1.0 | Undefined behavior in RNG sampling (negative probability) |
| `reproduction_threshold` | 0.0 | Instant reproduction every tick = population explosion |
| `reproduction_cooldown` | 0 | Chain-reaction reproduction = population explosion |
| `max_lifespan` | 0 | All creatures die immediately = empty sim |
| `food_energy` | 999999999 | Single food pellet fills all energy = breaks energy economy |
| `target_total_energy` | 0 | No food spawns ever = instant extinction |
| `season_amplitude` | 100.0 | Food rate goes negative = undefined behavior |
| `drag_coefficient` | -1.0 | Negative drag = acceleration feedback loop, velocities grow unbounded |
| `weight_clamp` | 0.0 | All neural weights clamped to zero = brain-dead creatures |

**This is the highest-severity finding in this audit.** Even without malicious intent, a user experimenting with extreme values can crash their own tab. If shareable URLs encode config parameters, a malicious link can crash any recipient's browser tab.

**Mitigation (MANDATORY):**
- **Validate ALL parameters in Rust `Simulation::new()` and `update_config()`**. The architecture mentions this but does not specify the validation rules. Every parameter MUST have a defined valid range.
- **Reject, do not clamp.** Return `Err(String)` for invalid configs so the UI can show what went wrong. Silent clamping hides bugs.
- **Validate in BOTH locations:** TypeScript (for immediate UI feedback) AND Rust (as the authoritative trust boundary). Never trust the JS validation alone.
- Proposed validation ranges:

```
world_width:              [100.0, 10000.0]
world_height:             [100.0, 10000.0]
initial_population:       [2, 5000]
max_energy:               [10.0, 10000.0]
food_energy:              [1.0, 1000.0]
target_total_energy:      [100.0, 1000000.0]
hidden_neurons:           [2, 32]
basal_cost:               [0.01, 100.0]
movement_cost_factor:     [0.0, 10.0]
reproduction_threshold:   [1.0, max_energy]
reproduction_energy_share:[0.1, 0.9]
maturation_period:        [1, 1000]
reproduction_cooldown:    [1, 10000]
max_lifespan:             [100, 100000]
drag_coefficient:         [0.001, 1.0]
max_turn_rate:            [0.01, PI]
base_mutation_rate:       [0.0, 1.0]
base_mutation_strength:   [0.0, 5.0]
season_amplitude:         [0.0, 0.9]
seed:                     [0, u64::MAX]  (all u64 values valid)
```

- **Compute a memory budget estimate** before allocating. For example:
  ```
  estimated_bytes = initial_population * (sizeof(Creature) + hidden_neurons * 12 * 4)
                  + (world_width/cell_size) * (world_height/cell_size) * sizeof(SmallVec)
                  + initial_population * FLOATS_PER_CREATURE * 4
  if estimated_bytes > 512MB { return Err("Configuration would exceed memory budget"); }
  ```

### F-4.2: Seed Input Safety

**Risk Level: LOW**

**Analysis:** The seed is a `u64` used to initialize `SmallRng`. All `u64` values are valid seeds -- there is no overflow, underflow, or special-case concern. `SmallRng::seed_from_u64()` accepts any `u64`.

The seed appears in shareable URLs. Since it is a numeric value with a fixed range, there is no injection risk from the seed itself. However, see F-4.3 for URL parameter handling.

**Mitigation:**
- Parse seed as `u64` strictly: reject non-numeric input, reject values outside `[0, 2^64-1]`.
- In TypeScript, `u64` exceeds `Number.MAX_SAFE_INTEGER`. Use `BigInt` for parsing or accept the seed as a string and pass it to WASM as two `u32` halves, or use `wasm-bindgen`'s `u64` support (which maps to `BigInt` in JS).

### F-4.3: URL Query Parameter Injection

**Risk Level: MEDIUM**

**Analysis:** Shareable URLs encode seed and config parameters. Attack vectors:

1. **XSS via URL parameters:** If any URL parameter value is inserted into the DOM without sanitization (e.g., displayed as "Seed: {params.seed}" using `dangerouslySetInnerHTML` or `document.innerHTML`), an attacker could craft a URL like `?seed=<script>alert(1)</script>`.

2. **Parameter pollution:** Multiple values for the same key (e.g., `?seed=42&seed=99`). Behavior depends on the URL parsing library.

3. **Prototype pollution:** If URL parameters are parsed into a plain object using naive key iteration, keys like `__proto__`, `constructor`, or `toString` could pollute the Object prototype. This is relevant if using a library like `qs` without sanitization.

4. **Config parameters as vectors:** If URL-encoded configs bypass the slider constraints and go directly to the WASM module, all the extreme-value attacks from F-4.1 apply via URL.

**Mitigation:**
- **Never render URL parameter values as raw HTML.** React's JSX default escaping handles this, but explicitly avoid `dangerouslySetInnerHTML` for any user-controlled value.
- **Parse URL parameters into a typed object with explicit allowlisted keys.** Ignore all unknown keys.
  ```typescript
  const ALLOWED_KEYS = new Set(['seed', 'world_width', 'world_height', ...]);
  const params = new URLSearchParams(window.location.search);
  const config: Partial<SimConfig> = {};
  for (const [key, value] of params.entries()) {
    if (!ALLOWED_KEYS.has(key)) continue;
    // Parse and validate each value
  }
  ```
- **Apply the same validation ranges as F-4.1** to URL-sourced config values.
- **Use `URLSearchParams` (built-in)**, not `qs` or manual parsing. `URLSearchParams` is not vulnerable to prototype pollution.
- **First-value wins** for duplicate keys: `params.get(key)` returns the first value.
- **Limit total URL length** to 2048 characters to prevent abuse.

---

## 5. Save/Load Binary Security

### F-5.1: Malformed Binary Save Files

**Risk Level: HIGH**

**Analysis:** Users can upload binary save files that are deserialized into simulation state within the WASM module. This is the highest-risk input surface because:

1. The binary blob is fully attacker-controlled.
2. Deserialization constructs complex in-memory data structures.
3. A crafted file could target the deserializer itself or the simulation logic that consumes the deserialized data.

**Attack vectors:**

- **Bincode deserialization exploits:** Bincode (the likely serialization format given the Rust ecosystem) has had specific security considerations:
  - **Bincode does NOT limit allocation sizes by default.** A Vec length prefix of `u64::MAX` will cause bincode to attempt to allocate that many bytes, triggering OOM.
  - Bincode's `deserialize` trusts length prefixes in the binary data. A crafted file with `Vec<Creature>` containing length = 10 million will allocate memory for 10 million creatures.
  - Bincode does NOT validate enum discriminants exhaustively in all configurations. An invalid discriminant can cause undefined behavior (though Rust's type system catches most cases).

- **Logic-level attacks after successful deserialization:**
  - Creatures with `energy = f32::INFINITY` (never die)
  - Creatures with `position` outside world bounds (spatial hash index out of bounds)
  - Genome with `hidden_neurons` count mismatching the brain weight array length
  - `genome_version` set to a future/unknown version
  - Negative values for fields that must be positive
  - NaN values in any f32 field (NaN propagates through all arithmetic, breaks comparisons)

- **Crafted save causing OOM:** A save file claiming a population of millions of creatures, each with thousands of hidden neurons.

**Mitigation (MANDATORY):**

- **Use `bincode::Options` with `with_limit()`** to cap the maximum deserializable size:
  ```rust
  use bincode::Options;
  let options = bincode::DefaultOptions::new()
      .with_limit(MAX_SAVE_SIZE_BYTES)  // e.g., 50MB
      .with_fixint_encoding();
  let state: SimState = options.deserialize(&bytes)?;
  ```
- **Validate ALL deserialized fields** post-deserialization. Every field must pass the same validation as `Simulation::new()`. Specifically:
  - Population count within bounds
  - All f32 values are finite (`f.is_finite()`)
  - All positions within world bounds
  - All energy values non-negative
  - Brain weight array lengths match `hidden_neurons` config
  - Genome version is supported
  - SlotMap keys are consistent (no dangling references)
- **Limit upload file size** in the UI: reject files larger than a reasonable maximum (e.g., 50MB) before passing to the Worker.
- **Deserialize inside the Worker** (already the case per architecture). If deserialization panics, only the Worker is affected; the main thread remains responsive.
- **Do NOT use `serde_json` or any text-based format for save files.** Stick with binary (bincode) to avoid JSON-specific attacks (deep nesting, huge strings).

### F-5.2: Decompression Bombs

**Risk Level: MEDIUM**

**Analysis:** If save files are compressed (e.g., with `flate2` / zlib / zstd), a decompression bomb is possible: a small compressed file (e.g., 1KB) that decompresses to gigabytes of data (e.g., repeated zero bytes).

**Mitigation:**
- **Limit decompressed output size.** Use streaming decompression with a byte counter:
  ```rust
  let mut decoder = flate2::read::DeflateDecoder::new(&compressed_bytes[..]);
  let mut output = Vec::with_capacity(expected_size);
  let bytes_read = decoder.take(MAX_DECOMPRESSED_SIZE).read_to_end(&mut output)?;
  if bytes_read >= MAX_DECOMPRESSED_SIZE { return Err("Decompression bomb detected"); }
  ```
- **Check compression ratio.** If `decompressed_size / compressed_size > 100`, reject as suspicious.
- Apply the bincode size limit (F-5.1) AFTER decompression as a second layer of defense.

### F-5.3: Genome Version Mismatch Exploitation

**Risk Level: LOW**

**Analysis:** The genome structure includes a `genome_version: u8` field for future migration. If a save file contains an unknown genome version, the deserializer must handle it gracefully.

**Attack scenario:** Set `genome_version = 255`. If the migration code uses a lookup table or match statement and does not handle unknown versions, it could panic or read out-of-bounds.

**Mitigation:**
- Match on `genome_version` with a catch-all arm that returns an error:
  ```rust
  match genome_version {
      1 => { /* current format */ },
      v => return Err(format!("Unsupported genome version: {}", v)),
  }
  ```
- Do NOT attempt to "guess" or "auto-upgrade" unknown versions.

---

## 6. IndexedDB Security

### F-6.1: Cross-Origin IndexedDB Access

**Risk Level: LOW**

**Analysis:** IndexedDB is same-origin isolated. A database created by `darwins-sandbox.vercel.app` is NOT accessible by any other origin. This is enforced by the browser engine and cannot be bypassed by JavaScript.

Exception: If the application is served from `localhost` during development, other localhost applications on the same port share the origin and could access the IndexedDB. This is a development-only concern.

**Mitigation:** None required for production. During development, use a unique database name prefix.

### F-6.2: Storage Quota Abuse

**Risk Level: LOW**

**Analysis:** If the auto-save feature writes large simulation states to IndexedDB on every save, and the simulation state is large (thousands of creatures with full genomes), storage could grow significantly. Modern browsers allocate per-origin quotas (typically up to 50% of available disk, but capped by the browser). Exceeding quota triggers a `QuotaExceededError`.

The application cannot abuse OTHER sites' storage because IndexedDB is origin-isolated. It can only fill its own quota.

**Mitigation:**
- **Limit the number of auto-save slots** (e.g., keep only the 3 most recent auto-saves, delete older ones).
- **Show storage usage** in the UI so users know how much space is consumed.
- **Catch `QuotaExceededError`** gracefully and notify the user instead of silently failing.
- **Estimate save size before writing.** If a single save would exceed a reasonable limit (e.g., 100MB), warn the user.

### F-6.3: IndexedDB Data Corruption

**Risk Level: LOW**

**Analysis:** IndexedDB transactions are atomic -- they either fully commit or fully roll back. However, data can become corrupted if:
- The browser crashes mid-transaction (rare, but possible)
- The IndexedDB implementation has bugs (very rare in modern browsers)
- A WASM panic occurs during save serialization, producing a partial/invalid blob

**Mitigation:**
- **Validate loaded data** with the same checks as F-5.1 (treat IndexedDB data with the same suspicion as uploaded files, since it could be corrupted).
- **Include a checksum** (e.g., CRC32 or xxHash) in the saved blob. Verify on load.
- **Use a single `put()` transaction** for saves, not multiple sequential writes.

---

## 7. Canvas/Rendering Security

### F-7.1: Canvas Fingerprinting

**Risk Level: LOW**

**Analysis:** Canvas fingerprinting is a technique where a website draws specific text/shapes on a hidden canvas and reads back the pixel data to create a unique fingerprint based on the user's GPU, drivers, and rendering engine. Darwin's Sandbox uses Canvas 2D for rendering the simulation, but:

- The application does NOT read back pixel data (`getImageData()`, `toDataURL()`).
- The application does NOT need to identify users.
- Even if `getImageData()` were called, it would return the simulation rendering, not a fingerprinting payload.

**Relevance:** Canvas fingerprinting is a concern for privacy-focused users who may see the Canvas usage in privacy extensions' reports. This is a false positive in context.

**Mitigation:**
- Do not call `getImageData()` or `toDataURL()` unless explicitly needed for a feature (e.g., screenshot export).
- If screenshot export is added, document it in the privacy policy.

### F-7.2: Rendering Malicious Data from WASM

**Risk Level: LOW**

**Analysis:** The render buffer from WASM contains Float32Array data: `[x, y, rotation, size, energy_norm, r, g, b]` per creature. If WASM produces invalid values:

| Value | Canvas Behavior |
|-------|----------------|
| NaN coordinates | Draw call becomes no-op (NaN fails all range checks) |
| Infinity coordinates | Draw call becomes no-op or clips to canvas bounds |
| Negative size | Path2D with negative dimensions may draw nothing or mirror |
| NaN color values | `fillStyle` set to invalid value defaults to black |
| Values > canvas size | Drawing occurs off-screen, invisible, no crash |

Modern Canvas 2D implementations handle all these edge cases gracefully. No crashes, no security impact.

**Mitigation:**
- No security mitigation needed.
- For correctness, optionally validate that render buffer values are finite before rendering, but this is a quality concern, not a security concern.

### F-7.3: Canvas Memory Consumption

**Risk Level: LOW**

**Analysis:** Canvas memory is proportional to `width * height * 4 bytes (RGBA)`. With DPI scaling, a 1920x1080 canvas at 2x DPI = 3840x2160 pixels = ~33MB. This is normal and well within browser limits.

The risk would increase only if the canvas is resized to extreme dimensions programmatically, which is not a feature in this application.

**Mitigation:** Cap canvas resolution at a reasonable maximum (e.g., 4096x4096 pixels after DPI scaling).

---

## 8. Supply Chain Security

### F-8.1: Rust Crate Dependencies

**Risk Level: MEDIUM**

| Crate | Version (from arch) | Known Issues | Risk |
|-------|---------------------|--------------|------|
| `wasm-bindgen` | latest | No known CVEs. Maintained by Rust wasm WG. | Low |
| `slotmap` | 1.0+ | No known CVEs. Stable, well-tested arena allocator. | Low |
| `rand` | 0.8 | No known CVEs. `SmallRng` is not cryptographic -- do not use for security purposes (not applicable here). | Low |
| `smallvec` | latest | **HISTORICAL: CVE-2019-15551 and CVE-2019-15554** (heap buffer overflow due to incorrect capacity handling). Fixed in smallvec 0.6.10+ and all 1.x versions. Ensure version >= 1.0. | Low (if version >= 1.0) |
| `serde` | latest | No known CVEs in serde core. | Low |
| `bincode` | latest | No CVEs, but see F-5.1 for unlimited allocation footgun. | Medium (usage, not crate bug) |
| `console_error_panic_hook` | latest | No known CVEs. Minimal crate. | Low |

**Mitigation:**
- **Run `cargo audit` in CI** on every build. This checks the RustSec Advisory Database.
- **Run `cargo vet`** to establish a supply chain verification policy. At minimum, vet all direct dependencies.
- **Pin exact versions** in `Cargo.lock` and commit it to version control.
- **Verify `smallvec` version >= 1.0.** The historical CVEs are in 0.x versions only.
- **Minimize dependency tree.** Use `cargo tree` to inspect transitive dependencies. The current dependency list is lean -- keep it that way.
- **Audit wasm-opt (Binaryen):** This is a build-time-only tool that transforms the WASM binary. Ensure it is installed from the official Binaryen releases, not an untrusted source.

### F-8.2: npm Dependencies

**Risk Level: MEDIUM**

| Package | Known Issues | Risk |
|---------|--------------|------|
| `next` (14.x) | Multiple CVEs historically in Next.js (SSRF, middleware bypass). Most are server-side. App Router client-only usage significantly reduces surface. | Low (client-only) |
| `comlink` | No known CVEs. Small library (~4KB). Google Chrome Labs maintained. | Low |
| `zustand` | No known CVEs. Minimal state management library. | Low |
| `uplot` | No known CVEs. Canvas-based charting, no DOM manipulation with user data. | Low |
| `recharts` | No known CVEs in core. Uses SVG rendering. Ensure data values are numeric, not strings that could be SVG injection. | Low |
| `tailwindcss` | Build-time only. No runtime security surface. | Low |

**Mitigation:**
- **Run `npm audit` / `pnpm audit` in CI.**
- **Use `pnpm` with strict peer dependencies** (default in pnpm) to prevent phantom dependencies.
- **Enable lockfile (`pnpm-lock.yaml`) and commit it.**
- **Use Dependabot or Renovate** for automated dependency updates.
- **Review Next.js security advisories** before upgrading major versions. Since this is client-only, most Next.js CVEs (which target the server-side) do not apply.
- **Do not install unnecessary dependencies.** The current list is appropriately minimal.

### F-8.3: Build Pipeline Integrity

**Risk Level: LOW**

**Analysis:** The build pipeline (`scripts/build-wasm.sh`) runs `wasm-pack build` and `wasm-opt`, then copies artifacts. If the build machine is compromised, the WASM binary could be tampered with. This is a general supply chain concern, not specific to this project.

**Mitigation:**
- Build in CI (e.g., GitHub Actions) with pinned tool versions.
- Use hash-verified tool installations (e.g., `cargo install wasm-pack --version X.Y.Z`).
- Consider Subresource Integrity (SRI) for the WASM binary if served from a CDN, though this is complex for dynamically fetched WASM.

---

## 9. Cross-Origin and Network Security

### F-9.1: SharedArrayBuffer and COOP/COEP

**Risk Level: MEDIUM**

**Analysis:** The architecture does NOT use `SharedArrayBuffer` -- it uses transferable `ArrayBuffer` (ownership transfer via `postMessage`). Therefore, COOP/COEP headers are **not strictly required** for the application to function.

However, COOP/COEP headers are still RECOMMENDED because:
1. They enable cross-origin isolation, which provides Spectre mitigations.
2. They are required if you ever want to use `SharedArrayBuffer` in the future (e.g., for the double-buffer optimization or high-resolution `performance.now()`).
3. They demonstrate security hygiene.

**Caveats of enabling COOP/COEP:**
- All cross-origin resources (images, scripts, fonts) must be served with `Cross-Origin-Resource-Policy: cross-origin` or loaded via `crossorigin` attribute.
- Third-party iframes (ads, analytics) may break. Not relevant for this application.
- Vercel's CDN serves assets from the same origin, so this should work without issues.

**Mitigation:** Add these headers in `next.config.js`:
```
Cross-Origin-Opener-Policy: same-origin
Cross-Origin-Embedder-Policy: require-corp
```

### F-9.2: Content Security Policy (CSP)

**Risk Level: MEDIUM**

**Analysis:** A strong CSP prevents XSS and code injection. For Darwin's Sandbox with WASM + Workers:

**Recommended CSP:**
```
Content-Security-Policy:
  default-src 'self';
  script-src 'self' 'wasm-unsafe-eval';
  worker-src 'self' blob:;
  style-src 'self' 'unsafe-inline';
  img-src 'self' data: blob:;
  connect-src 'self';
  font-src 'self';
  object-src 'none';
  base-uri 'self';
  form-action 'self';
  frame-ancestors 'none';
```

Key directives:
- **`'wasm-unsafe-eval'`**: Required for WASM compilation. This is the modern, minimal CSP directive for WASM (replaces the overly broad `'unsafe-eval'`). Supported in Chrome 103+, Firefox 102+, Safari 16+.
- **`worker-src 'self' blob:`**: Required for Web Worker creation. Webpack may generate blob: URLs for Workers.
- **`'unsafe-inline'` for styles**: Required by Tailwind CSS and most CSS-in-JS solutions. Can be tightened with nonces if desired.
- **`object-src 'none'`**: Prevents Flash/plugin-based attacks.
- **`frame-ancestors 'none'`**: Prevents clickjacking (equivalent to `X-Frame-Options: DENY`).

**Mitigation:** Configure these headers in `next.config.js` under the `headers()` function.

### F-9.3: Additional Next.js Security Headers

**Risk Level: LOW**

**Recommended headers:**
```
X-Content-Type-Options: nosniff
X-Frame-Options: DENY
Referrer-Policy: strict-origin-when-cross-origin
Permissions-Policy: camera=(), microphone=(), geolocation=()
Strict-Transport-Security: max-age=63072000; includeSubDomains; preload
X-DNS-Prefetch-Control: off
```

Next.js provides some of these by default. Verify they are all present in the production deployment.

**Mitigation:** Add all headers in `next.config.js`:
```javascript
async headers() {
  return [
    {
      source: '/(.*)',
      headers: [
        { key: 'X-Content-Type-Options', value: 'nosniff' },
        { key: 'X-Frame-Options', value: 'DENY' },
        { key: 'Referrer-Policy', value: 'strict-origin-when-cross-origin' },
        { key: 'Permissions-Policy', value: 'camera=(), microphone=(), geolocation=()' },
        { key: 'Content-Security-Policy', value: '...' },
        // COOP/COEP if desired
      ],
    },
  ];
}
```

### F-9.4: WASM Binary MIME Type

**Risk Level: LOW**

**Analysis:** The WASM binary must be served with `Content-Type: application/wasm`. If served with an incorrect MIME type, browsers with `X-Content-Type-Options: nosniff` will refuse to compile it. Vercel and Next.js handle `.wasm` MIME types correctly by default.

**Mitigation:** Verify that `public/wasm/*.wasm` files are served with `Content-Type: application/wasm` in production. Add a smoke test that fetches the WASM URL and checks the Content-Type header.

---

## 10. Denial of Service (Self-Inflicted)

### F-10.1: Population Explosion

**Risk Level: HIGH**

**Analysis:** The most likely self-inflicted DoS is population explosion. If the reproduction logic has a bug, or if configs allow it, the population can grow exponentially:

- Tick 0: 100 creatures
- Each creature reproduces once: 200 creatures
- Next tick: 400, then 800, then 1600...
- Within 20 ticks: over 100 million "intended" creatures

The closed energy budget is designed to prevent this, but edge cases exist:
- `reproduction_threshold` set too low
- `reproduction_cooldown` set to 0 or 1
- `reproduction_energy_share` set very low (0.1 means parent keeps 90%, can reproduce again almost immediately)
- `food_energy` set extremely high (one food pellet allows multiple reproductions)

**Memory impact per creature (estimated):**
- Creature struct: ~200 bytes (position, velocity, energy, age, etc.)
- Brain weights (8 hidden): 131 * 4 = 524 bytes
- Genome: ~600 bytes
- SlotMap overhead: ~16 bytes
- Total: ~1.3 KB per creature

At 100,000 creatures: ~130 MB. At 1,000,000: ~1.3 GB. Browser tab will crash.

**Mitigation (MANDATORY):**
- **Hard population cap in Rust.** If `creature_count >= MAX_CREATURES` (e.g., 10,000), skip all reproduction for that tick. This is a safety valve, not a design constraint.
  ```rust
  const MAX_CREATURES: usize = 10_000;
  if self.creatures.len() >= MAX_CREATURES {
      // Skip reproduction phase entirely
  }
  ```
- **Validate config interactions**, not just individual values. Specifically:
  - `reproduction_cooldown` must be >= `maturation_period / 2` to prevent runaway reproduction.
  - `reproduction_threshold` must be > `food_energy * 2` to ensure creatures must forage.
  - `reproduction_energy_share` must be >= 0.3 to prevent low-cost reproduction.
- **Monitor population in the Worker loop.** If population doubles within a single tick, log a warning and consider pausing.

### F-10.2: Spatial Hash Grid Memory

**Risk Level: MEDIUM**

**Analysis:** The spatial hash grid allocates `(world_width / cell_size) * (world_height / cell_size)` cells, each containing a `SmallVec<[SlotKey; 8]>`. If world dimensions are large and cell size is small:

- World 10000x10000, cell_size 60 (2x max vision): grid = 166 * 166 = ~28K cells. Each SmallVec inline = 8 * 8 = 64 bytes. Total: ~1.7 MB. Fine.
- World 10000x10000, cell_size 10: grid = 1000 * 1000 = 1M cells. Total: ~64 MB. Concerning.

Cell size is derived from `2 * max_vision_range`. If `max_vision_range` is set very small (e.g., 1.0) with a large world, the grid explodes.

**Mitigation:**
- **Enforce minimum cell size** of 20.0 regardless of vision range.
- **Enforce maximum grid dimensions:** `grid_width * grid_height <= 1,000,000` cells. If exceeded, increase cell_size.

### F-10.3: ArrayBuffer Size Limits

**Risk Level: LOW**

**Analysis:** Browser limits for individual ArrayBuffer allocations:
- Chrome: ~2 GB (limited by V8's heap and `kMaxLength`)
- Firefox: ~2 GB (limited by SpiderMonkey)
- Safari: ~4 GB (limited by JavaScriptCore)

The render buffer size = `creature_count * FLOATS_PER_CREATURE * 4 bytes`. At the proposed MAX_CREATURES of 10,000: `10,000 * 8 * 4 = 320 KB`. Well within limits.

The WASM linear memory can grow up to 4 GB (the WASM32 address space limit). In practice, browsers may fail to allocate before reaching this limit due to virtual address space fragmentation.

**Mitigation:** The MAX_CREATURES cap (F-10.1) and config validation (F-4.1) are sufficient. No additional ArrayBuffer-specific mitigation needed.

### F-10.4: Simulation Step Duration

**Risk Level: MEDIUM**

**Analysis:** If a single `step()` call takes too long (e.g., because population is large or spatial hash queries are degenerate), the Worker becomes unresponsive. The accumulator cap (`TICK_RATE * 5`) limits the number of ticks per frame, but each tick's duration is unbounded.

**Mitigation:**
- The heartbeat/watchdog mechanism from F-3.2 catches this case.
- **Profile and set a time budget for step().** If `step()` consistently exceeds 100ms, reduce simulation speed or skip frames.
- The architecture's Criterion benchmark target of <8ms per step for 1000 creatures provides a baseline.

---

## 11. Privacy

### F-11.1: Data Collection

**Risk Level: LOW**

**Analysis:** The architecture specifies no backend, no analytics, no user accounts. The application collects no personally identifiable information (PII) by design.

**Potential implicit data collection:**
- **Vercel Analytics:** If enabled (default in some Vercel templates), it collects page views, referrers, and geographic data. This is first-party analytics, not a security risk, but must be disclosed.
- **Error reporting:** If a service like Sentry is added, stack traces may contain user-specific data (URL with seed parameters, browser version).

**Mitigation:**
- Explicitly disable Vercel Analytics if not needed, or disclose it in a privacy notice.
- If error reporting is added, sanitize URLs (strip query parameters) before sending.

### F-11.2: Canvas Fingerprinting Exposure

**Risk Level: LOW**

**Analysis:** As discussed in F-7.1, the application does not perform canvas fingerprinting. However, privacy extensions (e.g., CanvasBlocker, Privacy Badger) may flag the Canvas usage. This is a user-experience concern, not a security concern.

**Mitigation:** No action required. If user reports come in about privacy extension warnings, add a note to the FAQ explaining that Canvas is used only for rendering the simulation.

### F-11.3: IndexedDB Data Persistence

**Risk Level: LOW**

**Analysis:** Simulation saves persist in IndexedDB across browser sessions. This data is:
- Not personally identifiable (it is simulation state: creature positions, genomes, etc.)
- Same-origin isolated (other sites cannot read it)
- Clearable by the user via browser settings or the application UI

**Mitigation:**
- Provide a "Clear all saves" button in the UI.
- Document in the privacy notice that simulation data is stored locally in IndexedDB.

### F-11.4: Shareable URL Privacy

**Risk Level: LOW**

**Analysis:** Shareable URLs contain seed and config parameters. These parameters:
- Do not contain PII.
- Are visible in browser history, referrer headers, and server logs.
- Could theoretically be used to reconstruct a user's simulation configuration (which is not sensitive).

**Mitigation:**
- Use `Referrer-Policy: strict-origin-when-cross-origin` (F-9.3) to prevent full URL leakage in referrer headers.
- If sharing URLs via a URL shortener is added in the future, ensure the shortener service's privacy policy is acceptable.

---

## 12. Summary Risk Matrix

| ID | Finding | Risk | Category | Requires Mitigation Before Ship |
|----|---------|------|----------|-------------------------------|
| F-4.1 | Extreme config values via DevTools/URL | **HIGH** | Input Validation | YES |
| F-5.1 | Malformed binary save file (OOM, invalid state) | **HIGH** | Binary Security | YES |
| F-10.1 | Population explosion crashing the tab | **HIGH** | Denial of Service | YES |
| F-3.2 | Runaway Worker (WASM infinite loop) | **MEDIUM** | Worker Isolation | YES |
| F-4.3 | URL query parameter injection/XSS | **MEDIUM** | Input Validation | YES |
| F-5.2 | Decompression bomb on save load | **MEDIUM** | Binary Security | YES (if compression used) |
| F-9.1 | Missing COOP/COEP headers | **MEDIUM** | Network Security | Recommended |
| F-9.2 | Missing or weak CSP | **MEDIUM** | Network Security | YES |
| F-10.2 | Spatial hash grid memory explosion | **MEDIUM** | Denial of Service | YES |
| F-10.4 | Simulation step duration unbounded | **MEDIUM** | Denial of Service | Recommended |
| F-8.1 | Rust crate supply chain (smallvec history) | **MEDIUM** | Supply Chain | YES (cargo audit in CI) |
| F-8.2 | npm supply chain | **MEDIUM** | Supply Chain | YES (pnpm audit in CI) |
| F-2.1 | WASM sandbox integrity | LOW | WASM Security | No (inherent to platform) |
| F-2.2 | WASM panic behavior | LOW | WASM Security | No (handled by architecture) |
| F-2.3 | WASM sandbox escape | LOW | WASM Security | No (browser-level concern) |
| F-2.4 | Spectre/Meltdown timing attacks | LOW | WASM Security | No (irrelevant threat model) |
| F-2.5 | wasm-bindgen generated code | LOW | WASM Security | No |
| F-3.1 | Worker isolation boundary | LOW | Worker Isolation | No (inherent to platform) |
| F-3.3 | Malformed Worker messages | LOW | Worker Isolation | Recommended |
| F-3.4 | Worker origin security | LOW | Worker Isolation | No |
| F-4.2 | Seed input (u64 safety) | LOW | Input Validation | No |
| F-5.3 | Genome version mismatch | LOW | Binary Security | YES (cheap to implement) |
| F-6.1 | Cross-origin IndexedDB | LOW | Storage | No (inherent to platform) |
| F-6.2 | Storage quota abuse | LOW | Storage | Recommended |
| F-6.3 | IndexedDB data corruption | LOW | Storage | Recommended |
| F-7.1 | Canvas fingerprinting | LOW | Privacy | No |
| F-7.2 | Rendering malicious WASM data | LOW | Canvas | No |
| F-7.3 | Canvas memory consumption | LOW | Canvas | No |
| F-8.3 | Build pipeline integrity | LOW | Supply Chain | Recommended |
| F-9.3 | Missing security headers | LOW | Network Security | YES (trivial to implement) |
| F-9.4 | WASM MIME type | LOW | Network Security | Verify in production |
| F-11.1 | Implicit data collection | LOW | Privacy | Recommended |
| F-11.2 | Canvas fingerprinting exposure | LOW | Privacy | No |
| F-11.3 | IndexedDB persistence | LOW | Privacy | Recommended |
| F-11.4 | Shareable URL privacy | LOW | Privacy | No |

---

## 13. Required Security Tests

These tests MUST be implemented and pass before the corresponding features ship.

### 13.1 Config Validation Tests (Rust unit tests)

```
test_reject_world_width_zero
test_reject_world_width_negative
test_reject_world_width_exceeds_max
test_reject_population_exceeds_max
test_reject_negative_mutation_rate
test_reject_nan_energy_value
test_reject_infinity_food_energy
test_reject_zero_reproduction_cooldown
test_reject_drag_coefficient_negative
test_reject_hidden_neurons_exceeds_max
test_accept_all_default_values
test_accept_all_minimum_values
test_accept_all_maximum_values
test_reject_config_exceeding_memory_budget
```

### 13.2 Save/Load Security Tests (Rust unit tests)

```
test_reject_save_file_exceeding_size_limit
test_reject_save_with_invalid_creature_count
test_reject_save_with_nan_positions
test_reject_save_with_infinity_energy
test_reject_save_with_negative_energy
test_reject_save_with_out_of_bounds_positions
test_reject_save_with_unknown_genome_version
test_reject_save_with_mismatched_brain_weights
test_reject_truncated_save_file
test_reject_empty_save_file
test_reject_decompression_bomb (if compression used)
test_roundtrip_save_load_integrity
test_save_includes_checksum
test_load_validates_checksum
```

### 13.3 Population Safety Tests (Rust integration tests)

```
test_population_cannot_exceed_hard_cap
test_reproduction_respects_cooldown
test_reproduction_requires_energy_threshold
test_energy_conservation_across_reproduction
test_population_stabilizes_under_default_config
```

### 13.4 Worker Resilience Tests (TypeScript/Playwright)

```
test_worker_heartbeat_timeout_triggers_restart
test_worker_terminate_and_respawn
test_malformed_message_from_worker_handled_gracefully
test_render_buffer_wrong_length_rejected
```

### 13.5 URL Parameter Tests (TypeScript unit tests)

```
test_reject_unknown_url_parameters
test_reject_xss_payload_in_seed_parameter
test_reject_non_numeric_seed
test_reject_config_values_outside_valid_range
test_first_value_wins_on_duplicate_parameters
test_url_length_limit_enforced
test_url_params_produce_valid_config
```

### 13.6 CSP and Header Tests (Playwright/Integration)

```
test_csp_header_present_in_response
test_csp_allows_wasm_compilation
test_csp_allows_worker_creation
test_csp_blocks_inline_scripts
test_x_content_type_options_nosniff_present
test_x_frame_options_deny_present
test_wasm_binary_served_with_correct_mime_type
```

---

## 14. Residual Risk

After implementing all mitigations, the following residual risks remain. These are accepted risks that cannot be fully eliminated at the application level.

### 14.1 Browser Engine Vulnerabilities (Accepted)

WASM sandbox escapes, JIT compiler bugs, and rendering engine vulnerabilities are browser-level concerns. The application cannot mitigate these. Users are expected to keep browsers updated. **Risk: Low, Impact: High, Likelihood: Very Low.**

### 14.2 Supply Chain Compromise (Reduced, Not Eliminated)

`cargo audit` and `pnpm audit` detect known vulnerabilities but cannot detect zero-day supply chain attacks (e.g., a maintainer account compromise pushing malicious code). **Risk: Low, Impact: High, Likelihood: Very Low.**

### 14.3 Client-Side Tampering (Accepted)

Users can modify any client-side code via DevTools. Since there is no server-side state and no competitive element, this is not a meaningful security concern. A user who crashes their own tab via DevTools manipulation has only affected themselves. **Risk: N/A (self-harm only).**

### 14.4 Speculative Execution Side Channels (Accepted)

Spectre-class attacks via WASM remain theoretically possible despite browser mitigations. However, the application handles no sensitive cross-origin data, so there is nothing to leak. **Risk: Negligible.**

### 14.5 Denial of Service via Crafted Shareable URLs (Reduced)

Even with config validation, a malicious URL could set parameters to the maximum allowed values (e.g., `world_width=10000&initial_population=5000&hidden_neurons=32`), which produces a resource-intensive but technically valid simulation. The user's tab would be slow but should not crash. **Risk: Low.**

---

## Appendix A: Security Checklist for Each Milestone

| Milestone | Security Gate |
|-----------|--------------|
| M1: Rust foundation | Config validation with ranges (F-4.1). Hard population cap (F-10.1). All f32 inputs checked for `is_finite()`. |
| M2: Web Worker bridge | Heartbeat/watchdog protocol (F-3.2). Message type validation (F-3.3). |
| M3: Neural network | Weight clamp enforcement. Hidden neuron count validation. |
| M4: Mutation + speciation | Mutation rate clamping. No unbounded allocations in mutation. |
| M5: Environment | Season amplitude validation. Energy budget conservation assertion. |
| M6: Canvas renderer | DPI-scaled resolution cap (F-7.3). |
| M7: Controls + Zustand | TypeScript-side config validation (mirror of Rust validation). URL parameter parsing with allowlist (F-4.3). |
| M8: Stats dashboard | Ensure stats values do not enter `dangerouslySetInnerHTML`. |
| M9: Creature inspector | No XSS from genome/species display values. |
| M10: Interventions | Intervention parameters validated same as config. |
| M11: Save/load | Bincode size limit (F-5.1). Post-deserialization validation. Checksum (F-6.3). Decompression limit (F-5.2). File size limit in UI. |
| M12: Polish + deploy | CSP headers (F-9.2). COOP/COEP (F-9.1). All security headers (F-9.3). WASM MIME type verification (F-9.4). `cargo audit` + `pnpm audit` in CI (F-8.1, F-8.2). |

---

*End of Security Audit*
