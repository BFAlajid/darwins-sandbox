# Darwin's Sandbox — Failsafe Architecture

## Problem

Darwin's Sandbox is a long-running, stateful browser simulation. Unlike typical web apps where a page reload is cheap, a user may invest hours watching evolution unfold. Any crash, corruption, or silent degradation destroys that investment. The simulation also operates across a trust boundary (Rust/WASM in a Worker, JS on main thread, IndexedDB for persistence) where each layer can fail independently.

This document defines every failure mode we can anticipate, how to detect it, how to prevent it, how to recover from it, and how to communicate it to the user.

## Assumptions

1. The simulation runs in a single Web Worker with a single WASM instance.
2. Target browsers: Chrome 90+, Firefox 90+, Safari 15+. No IE or legacy support.
3. Typical session: 30 minutes to several hours of continuous simulation.
4. Maximum target: 5,000 creatures (Canvas 2D limit before needing WebGL).
5. Users will run this on hardware ranging from modern desktops to low-end laptops and tablets.
6. No server component. All state lives in the browser.
7. IndexedDB is the only persistence layer.
8. The WASM module uses dlmalloc and has no threading (single-threaded WASM).

---

## 1. Simulation Stability Failsafes

### 1.1 Population Explosion

**Failure:** Population exceeds carrying capacity due to parameter misconfiguration, energy budget bug, or edge case where reproduction cascades faster than starvation can cull.

**Detection:**
- In Rust: check `creature_count` after every reproduction event.
- Soft threshold at 3,000. Hard cap at 5,000.

**Prevention:**
- The closed energy budget is the primary regulator. If total energy is conserved, population is self-limiting.
- Reproduction cooldown (120 ticks) and maturation (40 ticks) prevent chain reactions.
- Energy audit every 100 ticks (see 1.4) catches budget drift before it causes explosions.

**Recovery:**
- **Soft cap (3,000):** Increase basal metabolic cost by 50% temporarily. Log a warning. This applies natural pressure without abruptly killing creatures.
- **Hard cap (5,000):** Reject all reproduction attempts. No new creatures spawn until population drops below 4,000. Emit a `PopulationCapped` event to the Worker for UI notification.
- Never randomly cull. Abrupt culling destroys evolutionary progress and confuses the user.

**User Communication:**
- Yellow warning badge: "Population pressure: food is scarce" (at soft cap).
- Orange warning badge: "Population limit reached: reproduction paused" (at hard cap).

### 1.2 Population Collapse to Zero

**Failure:** All creatures die. Simulation becomes a static empty world.

**Detection:**
- Check `creature_count == 0` at the end of every tick.

**Prevention:**
- The 3:1 rule (creature survives 3x longer than time to find food) calibrates against immediate mass starvation.
- Seasonal amplitude capped at 0.4 (food never drops below 60% of base).
- Automated catastrophes only halve food (never eliminate it) and have a fixed duration.

**Recovery:**
- **Immediate:** Pause the simulation. Do not auto-restart silently.
- **Offer the user three choices:**
  1. "Reseed" -- spawn `initial_population` new random creatures (fresh genomes). Preserves world state, food, tick counter. Resets generation counter.
  2. "Revert to last auto-save" -- load the most recent auto-save checkpoint.
  3. "Reset" -- full simulation reset to initial conditions.
- If the user has not interacted within 5 seconds and the simulation is in "auto" mode (no user present), auto-reseed to keep things alive.

**User Communication:**
- Modal overlay: "All creatures have died. Evolution has ended." with the three recovery options.

### 1.3 NaN Propagation

**Failure:** A neural network produces NaN (e.g., from extreme weight values, division by zero in distance calculation, or accumulated floating point errors). NaN propagates through position, energy, and infects other creatures via neighbor interactions.

**Detection:**
- **Per-creature NaN guard in the hot loop.** After the neural network forward pass and physics integration, check:
  ```
  if position.x.is_nan() || position.y.is_nan() || energy.is_nan() || velocity.x.is_nan()
  ```
- This is a branch prediction-friendly check (NaN is extremely rare, so the branch is almost never taken). Cost: negligible.

**Prevention:**
- Weight clamping to [-5.0, 5.0] prevents tanh saturation and extreme products.
- Distance calculations use `1.0 / (1.0 + distance / vision_range)` which cannot divide by zero.
- All sigmoid/tanh inputs are bounded by design of the network topology.
- Energy subtraction floors at 0.0 (creature dies rather than going negative).

**Recovery:**
- If NaN is detected on any creature field: **kill that creature immediately**. Log its genome for debugging. Increment a `nan_deaths` counter.
- If `nan_deaths > 10` in a single tick: halt simulation, enter Error state. This indicates a systemic bug, not a one-off.
- NaN creatures are removed before any neighbor queries, preventing propagation.

**User Communication:**
- Single NaN death: silent (logged to diagnostics panel only).
- Systemic NaN outbreak: error modal with option to revert to last save.

### 1.4 Energy Budget Drift

**Failure:** Floating point accumulation errors cause total world energy to drift over millions of ticks. Budget gradually inflates (population explosion) or deflates (mass starvation).

**Detection:**
- **Energy audit every 100 ticks:**
  ```
  actual_energy = sum(creature.energy) + food_count * food_energy_value
  drift = actual_energy - target_total_energy
  drift_ratio = abs(drift) / target_total_energy
  ```
- Warn at `drift_ratio > 0.01` (1%). Correct at `drift_ratio > 0.02` (2%).

**Prevention:**
- Use a single authoritative `target_total_energy` value. Food spawning is always computed as:
  ```
  deficit = target_total_energy - measured_actual_energy
  ```
  This is inherently self-correcting because it recomputes from ground truth every spawn cycle.

**Recovery:**
- At 2% drift: adjust food spawning for the next cycle to compensate. If `drift > 0` (too much energy), spawn fewer food. If `drift < 0`, spawn more.
- At 5% drift (should never happen): force-recompute all creature energies by rounding to nearest 0.25 and adjusting food count to match target. Log as a warning.

**User Communication:**
- Silent correction. Energy drift is an internal bookkeeping concern, not user-facing.

### 1.5 Spatial Hash Corruption

**Failure:** A creature's position lands outside world bounds (negative coordinates, exceeding world dimensions) and maps to an invalid cell index.

**Detection:**
- Spatial hash `insert()` validates cell index: `assert!(cell_x < grid_width && cell_y < grid_height)`.
- In debug builds, this panics. In release builds, it skips the creature and logs.

**Prevention:**
- Toroidal wrapping (`rem_euclid`) is applied in the physics step, before spatial hash insertion.
- Position is re-wrapped after any external modification (teleportation from interventions, load from save).

**Recovery:**
- If a creature is outside bounds: force-wrap its position using `rem_euclid`. This is always safe for a toroidal world.
- If the spatial hash itself is corrupted (cell contains keys for dead creatures): the clear-and-rebuild-every-tick strategy means corruption never persists beyond one tick.

### 1.6 Food Spawn Goes Negative

**Failure:** The energy budget formula produces a negative food spawn count (more energy exists than the target).

**Detection:**
- The formula already includes `max(0, deficit / food_energy_value)`.

**Prevention:**
- `max(0, ...)` floor on food spawning.
- When population is high and total creature energy exceeds target, simply spawn zero food. The carrying capacity mechanism handles the rest.

**Recovery:**
- No recovery needed. Zero food spawn is the correct response to energy surplus. Starvation will naturally reduce the population.

---

## 2. WASM/Worker Failsafes

### 2.1 WASM Out-of-Memory

**Failure:** WASM linear memory cannot grow beyond the browser limit (typically 2-4 GB, but practically limited by the 32-bit address space to ~4 GB). `dlmalloc` returns null, Rust panics or aborts.

**Detection:**
- Rust: A failed allocation triggers a panic via the global allocator. With `console_error_panic_hook`, this surfaces as a readable error.
- Worker: catch the panic in a `try/catch` around the `step()` call.
- Proactive: monitor WASM memory size via `wasm.memory.buffer.byteLength` after each step. Alert at 512 MB. Hard-stop at 1 GB.

**Prevention:**
- Pre-allocate all major buffers (render buffer, spatial hash cells) with known maximums based on creature cap.
- Creature hard cap of 5,000 bounds the maximum memory usage to approximately:
  - 5,000 creatures x ~500 bytes each = ~2.5 MB
  - Spatial hash: ~1 MB
  - Render buffer: 5,000 x 8 floats x 4 bytes = 160 KB
  - Neural network weights are inline in creature structs
  - Total WASM heap: well under 50 MB even at maximum
- The real risk is memory leaks (dead creatures not freed, growing vectors never shrunk).

**Recovery:**
- On OOM panic: Worker catches the error, enters `Crashed` state, sends error to main thread.
- Offer user: "Simulation ran out of memory. Load last save or reset?"
- Do NOT attempt to continue. WASM memory state after OOM is unreliable.

**User Communication:**
- Error modal: "The simulation ran out of memory. Your last auto-save is available."

### 2.2 WASM Stack Overflow

**Failure:** Deep recursion in Rust code (unlikely with the current flat architecture, but possible if future features add recursive algorithms).

**Detection:**
- WASM stack overflow triggers a trap, which surfaces as a `RuntimeError` in JavaScript with message "call stack size exceeded" or similar.
- Worker catches this in the `try/catch` around `step()`.

**Prevention:**
- No recursive algorithms in the simulation. All iteration is flat (for loops over SlotMap).
- If recursion is ever introduced (e.g., tree-based spatial structures), use explicit stack with a depth limit.

**Recovery:**
- Same as OOM: enter `Crashed` state, offer save recovery.

### 2.3 Worker Unresponsive (Infinite Loop / Hung WASM)

**Failure:** WASM enters an infinite loop (bug in physics, pathological creature behavior, or corrupted state). The Worker stops responding to messages.

**Detection:**
- **Heartbeat/watchdog pattern from main thread:**
  ```
  Main thread sends: { type: 'ping', timestamp: Date.now() }
  Worker responds:   { type: 'pong', timestamp: <original> }
  ```
  - Main thread sends a ping every 2 seconds.
  - If no pong received within 5 seconds, declare the Worker unresponsive.
  - Important: the ping/pong must be processed in the Worker's message handler, which only runs between `step()` calls. If `step()` is blocked, the pong never sends. This is the desired detection behavior.

**Prevention:**
- Accumulator cap in the fixed-timestep loop: `accumulator = min(accumulator, TICK_RATE * 5)`. Prevents spiral-of-death where slow frames cause ever-more simulation steps.
- `step_n(N)` has a maximum N (e.g., 50). Even at max speed, the Worker processes at most 50 ticks per frame.
- Each `step()` call should complete within 16ms for 1,000 creatures. If a single step takes >100ms, something is wrong.

**Recovery:**
- Main thread terminates the Worker via `worker.terminate()`.
- Create a new Worker instance.
- Attempt to load the last auto-save into the new Worker.
- If the auto-save itself causes the hang (corrupted state), fall back to a fresh simulation.
- Preserve the corrupt save in a separate IndexedDB slot for debugging.

**User Communication:**
- Warning: "The simulation became unresponsive and was restarted. Your progress has been restored to the last auto-save."

### 2.4 Worker Crash Mid-Save

**Failure:** The Worker crashes or the tab closes while a save operation is in progress. The IndexedDB write is incomplete.

**Detection:**
- IndexedDB transactions are atomic. If the transaction does not complete, it is rolled back. There is no partial write.
- However, if we are serializing in WASM and the crash happens before the JS side initiates the IDB transaction, we lose the serialized bytes but the previous save remains intact.

**Prevention:**
- **Write-ahead pattern:** Never overwrite the current save. Always write to a new slot, then update the "latest" pointer atomically.
  ```
  Slot A: [valid save from 5 min ago]
  Slot B: [writing new save...]  <-- crash here
  Latest pointer: still points to Slot A
  ```
- Keep the last 3 save slots. Rotate through them.
- The "latest" pointer is updated only after the IDB transaction commits successfully.

**Recovery:**
- On next load, read the "latest" pointer. It always points to a fully committed save.
- If the pointer itself is missing (first run or total corruption), start fresh.

### 2.5 Unbounded WASM Memory Growth

**Failure:** Memory grows slowly over time due to leaks (vectors that grow but never shrink, fragmentation in dlmalloc).

**Detection:**
- Monitor `wasm.memory.buffer.byteLength` after each step.
- Track the delta over time. If memory grows by more than 1 MB per minute with a stable creature population, flag a potential leak.
- Report in diagnostics panel.

**Prevention:**
- `SlotMap` reuses slots. Dead creatures free their slots for reuse.
- Render buffer is pre-allocated with fixed capacity and rewritten in place.
- Spatial hash is cleared and rebuilt every tick (no growth).
- Periodic `Vec::shrink_to_fit()` on any dynamically sized vectors (e.g., species registry) every 1,000 ticks.

**Recovery:**
- If memory exceeds 256 MB (should be impossible with 5,000 creature cap): save state, terminate Worker, create new Worker, reload save. This effectively defragments memory.
- This is a last-resort measure. The real fix is finding the leak.

---

## 3. Rendering Failsafes

### 3.1 Canvas Context Lost

**Failure:** The browser reclaims the Canvas 2D context due to GPU memory pressure, driver crash, or device sleep/wake. The `contextlost` event fires and all subsequent draw calls silently fail.

**Detection:**
- Listen for `canvas.addEventListener('contextlost', handler)`.
- Listen for `canvas.addEventListener('contextrestored', handler)`.
- Also detect via: `ctx.isContextLost()` (available in modern browsers for 2D contexts as of 2024+).
- Fallback detection: if `getContext('2d')` returns null on a retry.

**Prevention:**
- Cannot be prevented. This is a browser/OS-level event.
- Minimize GPU memory usage: no offscreen canvases, no large image patterns.

**Recovery:**
- On `contextlost`: call `event.preventDefault()` (signals the browser we want to handle restoration).
- Set a `contextIsLost` flag. The rAF loop checks this flag and skips all draw calls.
- Display a static overlay: "Canvas context lost. Waiting for recovery..."
- On `contextrestored`: re-acquire context, reconfigure (DPI scaling, transforms), clear the `contextIsLost` flag. Drawing resumes automatically on next rAF.
- If context is not restored within 10 seconds: offer user a page reload.
- **Simulation continues in the Worker regardless.** Only rendering is affected.

**User Communication:**
- Overlay on canvas: "Display temporarily unavailable. Simulation continues in background."

### 3.2 requestAnimationFrame Stops (Tab Backgrounded)

**Failure:** When the user switches tabs, most browsers throttle or stop `requestAnimationFrame` callbacks. The render loop halts.

**Detection:**
- `document.addEventListener('visibilitychange', handler)`.
- `document.hidden` property.

**What happens to the simulation:**
- The Worker continues running. `setInterval`/`setTimeout` in Workers are throttled to ~1 per second in background tabs (Chrome), but the simulation loop should be driven by `setInterval` or a self-posting `postMessage` loop, not rAF.
- **Critical design decision:** The Worker simulation loop should use `setInterval(0)` with a time-based accumulator, NOT be driven by rAF messages from the main thread. This decouples simulation from rendering.

**Prevention/Handling:**
- When tab is hidden: stop the rAF loop on main thread. The Worker keeps simulating.
- When tab is visible again: restart rAF. The next frame will render the current state. There may be a visual "jump" as many ticks have elapsed.
- Optionally: throttle Worker simulation speed when tab is hidden to save battery/CPU. Send a `{ type: 'background' }` message to Worker, which reduces to 1 tick per 100ms.

**User Communication:**
- None needed. Seamless resume.

### 3.3 Stale or Corrupted Render Buffer

**Failure:** The render buffer received via `postMessage` contains corrupted data (wrong length, NaN values, creature count mismatch).

**Detection:**
- Validate buffer length: `buffer.byteLength === creatureCount * FLOATS_PER_CREATURE * 4`.
- Spot-check first and last creature for NaN: `isNaN(view[0]) || isNaN(view[1])`.
- Validate creature count is within expected range (0 to hard cap).

**Prevention:**
- The render buffer is a fixed-capacity pre-allocated array in Rust. Its length field is authoritative.
- The Worker always sends `{ buffer, creatureCount, foodCount }` as a consistent packet.

**Recovery:**
- If validation fails: skip this frame. Render the previous frame's data (double-buffer means we have it).
- If 10 consecutive frames fail validation: pause rendering and log an error. Simulation may be in a bad state.

### 3.4 GPU Memory Exhaustion

**Failure:** Very large canvas (e.g., 4K display, high DPI) causes GPU memory pressure.

**Detection:**
- There is no direct API for GPU memory. Detect indirectly via:
  - Canvas context loss (see 3.1).
  - Extremely low FPS (< 5 fps) sustained for multiple seconds.

**Prevention:**
- Cap canvas backing store size: `Math.min(displayWidth * dpr, 4096)`. Most GPUs handle 4096x4096 textures, but going beyond risks issues.
- On high-DPI displays, consider capping DPR at 2.0 even if the device reports 3.0.

**Recovery:**
- If context is lost and the canvas is very large: reduce DPR on recovery. Try `dpr = 1.0` and see if context stabilizes.

---

## 4. Data Integrity

### 4.1 Floating Point Determinism Across Browsers

**Research findings:**

- WASM specifies IEEE 754 single-precision (f32) and double-precision (f64) arithmetic with exact semantics for `+`, `-`, `*`, `/`, and `sqrt`.
- **Basic arithmetic IS deterministic across browsers.** `a + b` will produce the same f32 result in Chrome, Firefox, and Safari for identical inputs.
- **Where determinism breaks:**
  - `f32::sin()`, `f32::cos()`, `f32::exp()`, `f32::ln()` -- these are NOT specified to be bit-identical across implementations. WASM defers to the host's libm, which varies between browsers.
  - NaN bit patterns: WASM only guarantees "a NaN", not which specific NaN bit pattern. Different browsers may produce different NaN payloads.
  - Fused multiply-add (FMA): Some CPUs fuse `a * b + c` into a single operation with different rounding. WASM spec says implementations MAY fuse. Chrome V8 does on ARM; Firefox may not.

**Impact on Darwin's Sandbox:**
- `tanh` activation function uses `exp()` internally. **Cross-browser determinism is NOT guaranteed for neural network forward passes.**
- `sin()`/`cos()` used in seasonal variation and potentially in rotation calculations.

**Mitigation:**
- **Accept non-determinism across browsers.** Determinism guarantee is: same browser, same machine, same seed = same result. Document this.
- For `tanh`: consider a polynomial approximation (e.g., Pade approximant) that uses only `+`, `-`, `*`, `/`. This guarantees bit-identical results across all browsers. The approximation error is negligible for neural network activation. Example: `tanh_approx(x) = x * (27 + x*x) / (27 + 9*x*x)` is accurate to ~0.004 max error.
- For `sin`/`cos` (seasonal variation): these only affect food spawn rates, not creature behavior. Tiny differences are acceptable.
- **Recommended approach:** Use polynomial approximations for all transcendental functions in the hot path (brain forward pass). Use standard libm for everything else.
- If bit-exact cross-browser determinism ever becomes critical (e.g., for multiplayer or replay verification), implement all math functions as pure polynomial approximations.

### 4.2 Accumulation Errors Over Millions of Ticks

**Failure:** Positions, energies, and velocities accumulate floating point error over millions of additions.

**Detection:**
- Energy audit (see 1.4) catches energy drift.
- Position wrapping via `rem_euclid` inherently bounds position magnitude (never accumulates beyond world dimensions).
- Velocity is damped by drag every tick, preventing unbounded growth.

**Mitigation:**
- Energy: periodic audit and correction (see 1.4).
- Position: `rem_euclid` every tick is sufficient. Positions are always in `[0, world_width)` range.
- Velocity: drag coefficient of 0.08 means velocity decays toward zero every tick. Even if error accumulates, it is multiplicatively damped. Terminal velocity is naturally bounded by `thrust / drag`.
- **No explicit correction needed** beyond the energy audit. The physics model's damping and wrapping provide inherent stability.

### 4.3 Save/Load Roundtrip Fidelity

**Failure:** A saved simulation loads differently than expected due to serialization errors, version mismatches, or data corruption.

**Detection:**
- **Checksum on save:** Compute a CRC32 or xxHash of the serialized bytes. Store alongside the save.
- **Verify on load:** Recompute checksum and compare. Reject corrupted saves.
- **Tick-advance test:** After loading, run 10 ticks, serialize again, run 10 ticks from the save, compare. Must be identical (same-browser determinism).

**Save format:**
```
[magic_bytes: 4 bytes "DWSB"]
[format_version: u16]
[checksum: u32 (CRC32 of everything after this field)]
[compressed_payload: deflate-compressed bincode/MessagePack]
```

**Version migration:**
- `format_version` enables backward compatibility.
- Each version bump includes a migration function: `migrate_v1_to_v2(bytes) -> bytes`.
- Chain migrations for multi-version gaps.

**Recovery:**
- Checksum mismatch: reject the save. Display "Save file is corrupted."
- Version too new (downgrade): reject. "This save requires a newer version."
- Version too old: attempt migration chain. If migration fails, reject gracefully.

### 4.4 IndexedDB Write Failures

**Failure scenarios:**
- **Quota exceeded:** Browser storage limit reached (typically 50% of disk up to a max, varies by browser).
- **Private/incognito browsing:** Some browsers limit or disable IDB in private mode.
- **Safari restrictions:** IDB in Safari has known eviction behavior after 7 days of disuse.
- **Transaction aborted:** Browser kills long-running transactions.

**Detection:**
- Catch `DOMException` on IDB operations. Check `error.name`:
  - `QuotaExceededError`: storage full.
  - `AbortError`: transaction aborted.
  - `UnknownError`: general failure.
- On app startup, perform a probe write/read/delete to verify IDB is functional.
- `navigator.storage.estimate()` for quota monitoring.

**Prevention:**
- Keep saves small. Compress with deflate. A 5,000-creature simulation should be under 1 MB compressed.
- Rotate saves: keep only the last 3 auto-saves and up to 5 manual saves. Delete oldest when adding new.
- Request persistent storage: `navigator.storage.persist()` to prevent eviction.

**Recovery:**
- **Quota exceeded:** Delete oldest auto-saves. Retry. If still failing, warn user.
- **IDB unavailable (private browsing):** Detect on startup. Disable auto-save. Show banner: "Saves are unavailable in private browsing. Your simulation will not persist."
- **Transaction aborted:** Retry once after a 100ms delay. If it fails again, skip this save cycle and try next scheduled save.
- **Fallback:** Offer "Download save file" button that exports to a file via `Blob` + `URL.createObjectURL`. This works even when IDB is completely broken.

**User Communication:**
- IDB unavailable: persistent banner at bottom.
- Quota exceeded: "Storage is full. Oldest saves have been removed."
- Save failed: "Auto-save failed. Your simulation is still running. Retrying..."

---

## 5. Simulation Lifecycle State Machine

### 5.1 States

```
 Uninitialized --> Loading --> Running <--> Paused
                      |           |            |
                      |           v            v
                      |       Stepping ------->|
                      |           |
                      v           v
                    Error <--- Crashed
                      |
                      v
                  Recovering --> Running
                      |
                      v
                    Reset --> Loading
```

| State | Description |
|-------|-------------|
| `Uninitialized` | Initial state. No Worker, no WASM. |
| `Loading` | Worker created, WASM loading/initializing, or loading a save. |
| `Running` | Simulation actively ticking. |
| `Paused` | Simulation halted. World state preserved. User can inspect. |
| `Stepping` | Single-step mode. Advances one tick, then returns to Paused. |
| `Error` | A recoverable error occurred (NaN outbreak, validation failure). Simulation paused. |
| `Crashed` | Unrecoverable failure (OOM, Worker death, WASM trap). Worker may be dead. |
| `Recovering` | Loading a save into a new Worker after a crash. |

### 5.2 Valid Transitions

| From | To | Trigger |
|------|-----|---------|
| `Uninitialized` | `Loading` | User opens page / clicks "Start" |
| `Loading` | `Running` | WASM initialized successfully |
| `Loading` | `Error` | WASM load failed, save parse failed |
| `Running` | `Paused` | User clicks pause |
| `Running` | `Stepping` | (not directly -- must pause first) |
| `Running` | `Error` | NaN outbreak, validation failure |
| `Running` | `Crashed` | Worker died, OOM, WASM trap |
| `Paused` | `Running` | User clicks play |
| `Paused` | `Stepping` | User clicks "Step" |
| `Paused` | `Loading` | User loads a save or resets |
| `Stepping` | `Paused` | Step complete |
| `Stepping` | `Error` | Error during step |
| `Error` | `Recovering` | User clicks "Recover" / auto-recover |
| `Error` | `Loading` | User clicks "Reset" |
| `Crashed` | `Recovering` | Automatic or user-initiated |
| `Crashed` | `Loading` | User clicks "Reset" |
| `Recovering` | `Running` | Recovery successful |
| `Recovering` | `Error` | Recovery failed |

### 5.3 Invalid Transition Guard

All state transitions go through a single `transition(from, to)` function that validates against the table above. Invalid transitions are logged as errors and rejected. Example: attempting to `step()` while in `Loading` state is silently ignored.

```typescript
// In simulation store
function transition(to: SimState): boolean {
  const valid = VALID_TRANSITIONS[currentState];
  if (!valid.includes(to)) {
    console.error(`Invalid state transition: ${currentState} -> ${to}`);
    return false;
  }
  previousState = currentState;
  currentState = to;
  return true;
}
```

### 5.4 Recovery Matrix

| State | Can Recover? | Method |
|-------|-------------|--------|
| `Error` | Yes | Load last auto-save into existing Worker |
| `Crashed` | Partially | Terminate Worker, create new one, load last auto-save |
| `Crashed` (no saves) | No | Must reset to initial state |

---

## 6. Graceful Degradation

### 6.1 No Web Worker Support

**Detection:** `typeof Worker === 'undefined'`

**Fallback:** This is a hard requirement. Without Workers, the simulation blocks the main thread, making the UI unresponsive. Do NOT attempt a main-thread fallback.

**User Communication:** "Darwin's Sandbox requires Web Workers, which your browser does not support. Please use a modern browser (Chrome, Firefox, Safari, or Edge)."

### 6.2 No WASM Support

**Detection:** `typeof WebAssembly === 'undefined'`

**Fallback:** Hard requirement. No JavaScript fallback -- the simulation is too compute-intensive.

**User Communication:** "Darwin's Sandbox requires WebAssembly, which your browser does not support. Please use a modern browser."

### 6.3 Low-End Device / Cannot Maintain 60fps

**Detection:**
- Measure frame time over a rolling 60-frame window.
- If median frame time > 20ms (below 50fps), enter "adaptive quality" mode.
- If median frame time > 50ms (below 20fps), enter "low quality" mode.

**Adaptive Quality Tiers:**

| Tier | FPS Threshold | Actions |
|------|--------------|---------|
| **Full** | >= 50 fps | All features enabled |
| **Reduced** | 30-50 fps | Skip food rendering (draw as simple dots instead of circles). Reduce stats update frequency to 1Hz. Disable creature rotation in rendering (draw circles not triangles). |
| **Minimal** | 15-30 fps | Additionally: reduce canvas resolution (DPR = 1.0). Only render every 2nd frame. Reduce max creatures to 2,000. |
| **Emergency** | < 15 fps | Pause rendering entirely. Simulation continues. Show stats-only view. Offer "Reduce creature count?" prompt. |

**User Communication:**
- Subtle indicator: "Performance: [Full / Reduced / Minimal]" in the diagnostics panel.
- At Emergency tier: "Your device is struggling. Rendering has been paused. Click to resume at lower quality."

### 6.4 Mobile / Limited Memory

**Detection:**
- `navigator.deviceMemory` (Chrome only, returns GB of RAM, approximate).
- `navigator.hardwareConcurrency` for CPU core count.
- Screen size heuristic: if viewport width < 768px, assume mobile.

**Adjustments:**
- Mobile devices: reduce default creature cap to 1,000 (from 5,000).
- Low memory (< 4 GB): reduce to 2,000.
- Inform the user in settings that the creature cap has been adjusted and allow them to override (with a warning).

---

## 7. Rate Limiting and Throttling

### 7.1 Config Updates from UI

**Problem:** User rapidly drags a slider, generating dozens of config update messages per second.

**Mechanism:**
- Debounce slider changes: 100ms debounce on all parameter sliders.
- Batch config updates: collect all changed parameters, send a single `{ type: 'config', changes: {...} }` message.
- Worker applies config atomically at the start of the next tick.

### 7.2 Stats Updates from Worker

**Problem:** Worker sends stats every tick at max speed (potentially thousands per second at high sim speed), causing React re-render storms.

**Mechanism:**
- Worker throttles stats emission to a configurable rate: 2Hz at normal speed, 5Hz at high speed.
- Use a tick-count modulo: `if (tick % stats_interval === 0) { emit_stats() }`.
- Main thread Zustand store uses `getState().setState()` (non-reactive) for chart data buffers, only triggering re-renders at the throttled rate.

### 7.3 Save Frequency

**Mechanism:**
- Auto-save every 60 seconds OR every 500 ticks, whichever comes first.
- Never auto-save more than once per 30 seconds (hard floor).
- Manual save has a 5-second cooldown (disable button, show countdown).
- Saves happen in the Worker thread to avoid blocking the main thread.

### 7.4 Speed Slider and step_n(N) Budget

**Problem:** At 50x speed, the Worker calls `step_n(50)` per frame. If each step takes 8ms, that is 400ms per frame -- far exceeding the 16ms frame budget.

**Mechanism:**
- **Time-budgeted stepping:** Instead of a fixed `step_n(speed)`, the Worker measures elapsed time:
  ```
  const TICK_BUDGET_MS = 14; // leave 2ms headroom
  let ticks_done = 0;
  const start = performance.now();
  while (ticks_done < requested_ticks && (performance.now() - start) < TICK_BUDGET_MS) {
      sim.step();
      ticks_done++;
  }
  ```
- Report `actual_speed = ticks_done` to the UI. If the user requests 50x but only 20 ticks fit in the budget, display "Speed: 20/50 (limited by device)".
- This prevents the Worker from becoming unresponsive at high speeds.

---

## 8. Recovery Mechanisms

### 8.1 Auto-Save Before Interventions

**Mechanism:**
- Before any user-triggered intervention (meteor, drought, flood, wall drawing, predator mode), automatically save the current state.
- Label it: `"pre-intervention-{intervention_type}-tick-{tick}"`.
- Keep the last 3 intervention saves in addition to regular auto-saves.
- This effectively gives the user an "undo intervention" capability.

### 8.2 Last Known Good State

**Mechanism:**
- Maintain a `last_known_good` save slot that is updated only when the simulation has been running stably for at least 60 seconds (no errors, population > 10, no NaN deaths).
- This save is never overwritten by auto-saves that occur during unstable periods.
- Used as the fallback when the most recent auto-save is itself corrupted or causes crashes.

**Recovery chain:**
1. Try most recent auto-save.
2. If that fails/crashes: try `last_known_good`.
3. If that fails: try next-oldest auto-save.
4. If all fail: offer fresh reset only.

### 8.3 Corrupted IndexedDB Save Detection

**Detection:**
- Every save includes a CRC32 checksum (see 4.3).
- On load: verify checksum before deserialization.
- After deserialization: run a validation pass:
  - All creature positions within world bounds?
  - All energies >= 0 and not NaN?
  - Population count matches creature array length?
  - Tick counter is monotonically increasing vs. previous save?

**Recovery:**
- Corrupted save: delete it from IDB. Fall back to next save in the recovery chain.
- If the corruption is partial (checksum passes but validation fails): attempt to "heal" the save by clamping all values to valid ranges. If more than 10% of creatures are invalid, reject the save entirely.

### 8.4 Browser Crash Recovery

**What is preserved:**
- IndexedDB data survives browser crashes, OS crashes, and power loss (transactions are durable once committed).
- WASM memory, Worker state, and React state are all lost.

**On next page load:**
- Check IndexedDB for existing saves.
- If saves exist: offer "Continue where you left off?" with the most recent save's tick count and timestamp.
- If no saves: start fresh.

---

## 9. Resource Limits and Budgets

### 9.1 WASM Memory Cap

- **Browser limits:** Chrome allows up to 4 GB for WASM linear memory. Firefox and Safari similar. Practical limit for 32-bit WASM is ~4 GB.
- **Our cap:** Initial memory = 16 MB. Maximum memory = 256 MB (set in `Cargo.toml` or `wasm-bindgen` config via `memory` section).
- **Monitoring:** Check `wasm.memory.buffer.byteLength` every 100 ticks. Log if it exceeds 64 MB (unexpected for our use case).

### 9.2 Creature Cap

| Tier | Count | User Messaging |
|------|-------|----------------|
| Normal | 0-3,000 | No messaging |
| Warning | 3,000-4,500 | Yellow indicator: "High population" |
| Soft cap | 4,500-5,000 | Increased metabolic cost. "Population pressure active" |
| Hard cap | 5,000 | Reproduction blocked. "Maximum population reached" |

### 9.3 Simulation Speed Budget

| Speed Multiplier | Expected Ticks/Frame | Tick Budget | Notes |
|-----|-----|-----|-----|
| 1x | 1 | 16ms | Normal |
| 2x-5x | 2-5 | 16ms | Comfortable |
| 5x-20x | 5-20 | 14ms (time-budgeted) | May not reach target |
| 20x-50x | 20-50 | 14ms (time-budgeted) | Likely capped by device |
| Turbo | Unlimited | 14ms (time-budgeted) | "As fast as possible" mode. No rendering. |

### 9.4 IndexedDB Storage Budget

- **Detection:** `navigator.storage.estimate()` returns `{ usage, quota }`.
- **Warning at 80% quota.** "Storage is getting full. Consider deleting old saves."
- **Action at 90% quota:** Auto-delete oldest auto-saves (keep manual saves). Warn user.
- **Typical save size:** ~200 KB compressed for 1,000 creatures. ~800 KB for 5,000 creatures.
- **With 3 auto-saves + 5 manual saves + 3 intervention saves:** ~10 MB maximum. Well within typical browser quotas (hundreds of MB to GB).

---

## 10. Logging and Diagnostics

### 10.1 Structured Logging from WASM

WASM cannot directly log to the JS console. Use `wasm-bindgen` to expose a logging function:

**Log levels:**
- `Error`: NaN deaths, budget drift > 2%, validation failures. Always logged.
- `Warn`: Population caps hit, performance degradation, save failures. Always logged.
- `Info`: State transitions, save/load events, intervention triggers. Logged in dev, suppressed in prod.
- `Debug`: Per-tick stats, individual creature events. Only logged when diagnostics panel is open.

**Mechanism:**
- Rust calls `web_sys::console::log_1()` for critical errors (always available).
- For structured data, Rust populates a diagnostics struct that JS reads alongside the render buffer.
- Avoid logging in the hot loop. Batch diagnostics into a summary emitted every 100 ticks.

### 10.2 Performance Monitoring

**Metrics tracked (in Worker):**
- `tick_time_ms`: Time for a single `step()` call. Rolling average over 100 ticks.
- `ticks_per_frame`: Actual ticks achieved per render frame.
- `creature_count`: Current population.
- `memory_bytes`: WASM linear memory size.
- `food_count`: Current food items.
- `energy_total`: Actual total energy (for drift monitoring).

**Metrics tracked (in Main Thread):**
- `frame_time_ms`: Time between rAF callbacks.
- `render_time_ms`: Time spent in canvas draw calls.
- `fps`: Smoothed frames per second.
- `message_latency_ms`: Time between Worker posting a frame and main thread receiving it.

**Budget alerts:**
- If `tick_time_ms > 12ms` (75% of frame budget for 60fps): warn in diagnostics.
- If `tick_time_ms > 16ms`: the simulation cannot maintain real-time at 1x speed. Suggest reducing creature count.

### 10.3 User-Facing Diagnostics Panel

A toggleable panel (keyboard shortcut: `D` or button in controls) showing:

```
FPS: 60 | Tick: 8.2ms | Creatures: 847 | Food: 312
Memory: 24 MB | Speed: 5x (actual: 5x) | Gen: 142
Energy drift: 0.1% | NaN deaths: 0 | State: Running
Last save: 12s ago | Quality: Full
```

- All values updated at 2Hz (not 60fps).
- Color-coded: green = healthy, yellow = warning, red = critical.
- Collapsible to a single-line summary: "FPS: 60 | Pop: 847 | Gen: 142".

### 10.4 Error Event Log

Maintain a circular buffer of the last 100 error/warning events:

```
[tick 45230] WARN: Energy drift 1.2%, correcting
[tick 45180] WARN: Population soft cap reached (3,012)
[tick 44900] INFO: Auto-save completed (245 KB)
[tick 44100] ERROR: NaN detected in creature #2847, killed
```

Accessible via diagnostics panel. Exportable as text for bug reports.

---

## Summary: Defense-in-Depth Layers

```
Layer 1: Prevention
  - Closed energy budget prevents population instability
  - Weight clamping prevents NaN generation
  - Toroidal wrapping prevents out-of-bounds
  - Pre-allocated buffers prevent memory growth
  - Debounced/throttled messaging prevents flooding

Layer 2: Detection
  - Energy audit every 100 ticks
  - NaN guards on every creature every tick
  - Heartbeat watchdog on Worker
  - Frame time monitoring for performance
  - Checksum verification on saves
  - State machine transition validation

Layer 3: Correction
  - Energy budget auto-correction
  - NaN creature culling (before propagation)
  - Adaptive quality tiers
  - Time-budgeted stepping
  - Auto-recovery from Worker crash

Layer 4: Recovery
  - Write-ahead save pattern (3 rotating slots)
  - Last-known-good checkpoint
  - Pre-intervention saves (undo capability)
  - Save recovery chain (newest -> LKG -> oldest -> fresh)
  - Worker termination and respawn

Layer 5: Communication
  - Diagnostics panel with real-time metrics
  - State-specific error modals with actionable options
  - Performance tier indicators
  - Error event log for debugging
  - Never silent failures -- every error has a user-visible path
```

---

## Open Questions

1. **Polynomial tanh approximation:** What accuracy is acceptable for the neural network? Should we benchmark evolution quality with the approximation vs. libm? The Pade approximant `x*(27+x^2)/(27+9*x^2)` has max error ~0.004 which should be fine, but we should validate.

2. **Worker heartbeat granularity:** 2-second ping interval means up to 7 seconds to detect a hung Worker (2s interval + 5s timeout). Is this acceptable? Tighter intervals add overhead.

3. **Mobile battery impact:** Running a WASM simulation continuously drains battery. Should we add a "Battery saver" mode that auto-pauses after N minutes of inactivity? Detect `navigator.getBattery()` API?

4. **Safari IDB eviction:** Safari may evict IndexedDB data after 7 days of disuse. Should we warn Safari users to export saves manually? Or implement a "download save file" feature as primary persistence?

5. **SharedArrayBuffer alternative:** If SAB is available (requires COOP/COEP headers), we could share memory between Worker and main thread without copying. This eliminates the transferable buffer pattern entirely. Worth pursuing, or too complex for v1?

6. **Crash telemetry:** Since there is no server, crash data is lost. Should we offer an optional "copy diagnostics to clipboard" feature for users to include in GitHub issues?

7. **Maximum session length:** Should we recommend or enforce periodic saves and refreshes? After 24+ hours of continuous running, browser memory fragmentation may cause issues regardless of our safeguards.
