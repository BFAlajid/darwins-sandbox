# Darwin's Sandbox — Architecture Plan (v5 — Polished)

## Overview
Real-time evolution simulator where creatures with neural network brains compete for food, reproduce with mutations, and undergo natural selection in a browser sandbox. Rust/WASM simulation core, Next.js frontend.

## Tech Stack
- **Simulation Engine:** Rust compiled to WASM (`wasm32-unknown-unknown`), run in a dedicated Web Worker
- **Build Tool:** `wasm-bindgen` CLI directly (not wasm-pack wrapper) for maximum control over the thin WASM boundary. `wasm-opt` (Binaryen) for release optimization passes
- **wasm-pack Target:** `--target web` — generates ES module with manual `init()`, works in Workers without bundler dependency
- **Frontend:** Next.js 14 (App Router) with `--webpack` flag (Turbopack lacks WASM/Worker bundling support), TypeScript, Tailwind CSS
- **Rendering:** HTML Canvas 2D with path batching + LOD system. Upgrade path to thin WebGL instanced renderer (~200 LOC) if >3000 creatures needed.
- **State Bridge:** wasm-bindgen for Rust<->JS interop. Render data via direct pointer into WASM linear memory (zero-copy). Re-acquire `Float32Array` view after every `step()` call.
- **Worker Communication:** Comlink for RPC commands (start/stop/config). Raw `postMessage` with transferable `ArrayBuffer` for hot render data path.
- **Charts:** uPlot for live time-series (Canvas-based, 30KB, handles thousands of points at 60fps). Recharts acceptable for static summary views only.
- **Theme:** Dark mode only (dark gray background, high-contrast UI). No light mode — reduces design surface, matches "science lab" aesthetic.
- **State Management:** Zustand — stats throttled to 2-5Hz via `getState()`/`setState()`. Render data stored in `useRef`, never in React state.
- **Package Manager:** pnpm
- **RNG:** `SmallRng` (Xoshiro128++ on WASM's 32-bit target) via `rand = { version = "0.8", features = ["small_rng"] }`. No `thread_rng()` or `OsRng` in WASM.
- **Entity Storage:** `slotmap` v1.0+ — use `DenseSlotMap` from the start (better cache locality for iteration-heavy simulation loops)
- **Allocator:** Default (dlmalloc). Do NOT use `wee_alloc` (unmaintained, known memory leaks).
- **Serialization:** `bincode` with `Options::with_limit(MAX_SAVE_SIZE)` — never trust length prefixes from untrusted input
- **Compression:** `flate2` (deflate) with streaming decompression capped at `MAX_DECOMPRESSED_SIZE`
- **Panic Mode:** `panic = "abort"` in release profile — reduces binary size, eliminates unwind tables
- **Activation Function:** Polynomial `tanh` approximation `x*(27+x²)/(27+9x²)` — uses only deterministic `+`, `-`, `*`, `/` operations for cross-browser bit-identical results

## Project Structure

```
darwins-sandbox/
├── crates/
│   └── simulation/              # Rust simulation engine
│       ├── src/
│       │   ├── lib.rs            # WASM entry, #[wasm_bindgen(start)] panic hook, minimal exports
│       │   ├── world.rs          # World state, energy budget, food spawning, seasons
│       │   ├── creature.rs       # Creature struct, energy, lifecycle, reproduction
│       │   ├── brain.rs          # Feedforward NN (forward pass, weight clamping)
│       │   ├── genetics.rs       # Genome encoding, mutation (Gaussian + Cauchy), trait mapping
│       │   ├── physics.rs        # Momentum, drag, turning, collision, spatial hash
│       │   ├── species.rs        # Speciation: genomic distance, dynamic threshold, species registry
│       │   ├── config.rs         # All tunable parameters with validation + defaults
│       │   ├── render_buffer.rs  # Pre-allocated flat f32 buffer for zero-copy export, sorted by species
│       │   ├── events.rs         # Evolution event detection (speciation, extinction, records)
│       │   └── profile.rs        # Tick phase timing instrumentation
│       ├── Cargo.toml
│       │   └── validation.rs     # Config validation, save validation, NaN guards
│       └── tests/
│           ├── determinism.rs    # Same seed = same result after N ticks
│           ├── stability.rs      # Population doesn't explode or collapse
│           ├── food_taxis.rs     # Avg food distance decreasing over generations
│           └── security.rs       # Config boundary tests, save fuzz, NaN injection
├── src/
│   ├── app/
│   │   ├── layout.tsx
│   │   ├── page.tsx              # Main sandbox page ('use client')
│   │   └── globals.css
│   ├── components/
│   │   ├── canvas/
│   │   │   ├── simulation-canvas.tsx   # Canvas element + rAF loop reading from useRef
│   │   │   ├── renderer.ts             # Path-batched drawing with LOD + interpolation
│   │   │   ├── lod.ts                  # LOD tier calculation from zoom level
│   │   │   ├── particles.ts            # Client-side particle pool (birth/death effects)
│   │   │   ├── minimap.ts              # 120x90 minimap rendered at 2Hz
│   │   │   └── data-lens.ts            # Color mapping modes (species/energy/speed/age/mutation)
│   │   ├── controls/
│   │   │   ├── simulation-controls.tsx # Play/pause, step, speed slider, fast-forward
│   │   │   ├── parameter-panel.tsx     # Tunable params with live update
│   │   │   └── intervention-tools.tsx  # Meteor, drought, flood, wall drawing
│   │   ├── sidebar/
│   │   │   ├── species-panel.tsx       # Species list with sparklines + population %
│   │   │   ├── creature-inspector.tsx  # Selected creature brain/genome/stats view
│   │   │   └── species-chronicle.tsx   # Auto-generated species narrative summary
│   │   ├── charts/
│   │   │   ├── population-chart.tsx    # uPlot: live population per-species stacked area
│   │   │   ├── trait-distribution.tsx  # uPlot: speed/size/vision over time
│   │   │   └── event-log.tsx           # Narration log with scrollback
│   │   └── ui/
│   │       ├── slider.tsx
│   │       ├── button.tsx
│   │       ├── panel.tsx
│   │       ├── toast.tsx               # Achievement/notification toasts
│   │       └── keyboard-shortcuts.tsx  # ? menu overlay
│   ├── lib/
│   │   ├── wasm-bridge.ts        # Comlink wrapper for Worker RPC
│   │   ├── types.ts              # Shared TypeScript types
│   │   └── utils.ts
│   ├── workers/
│   │   ├── simulation.worker.ts  # Web Worker: owns WASM, handles commands, posts render buffers
│   │   └── wasm/                 # wasm-bindgen JS glue + .d.ts copied here by build script
│   ├── stores/
│   │   └── simulation-store.ts   # Zustand store for stats (throttled), config, UI state, events, species names
│   └── hooks/
│       ├── use-simulation.ts     # Init Worker, command dispatch, cleanup, watchdog
│       ├── use-canvas.ts         # Canvas ref, resize observer, DPI scaling
│       ├── use-keyboard.ts       # Global keyboard shortcut handler
│       └── use-interpolation.ts  # Frame interpolation between sim ticks
├── public/
│   └── wasm/                     # .wasm binary served as static asset
├── scripts/
│   └── build-wasm.sh            # wasm-pack build + copy to public/ and src/workers/wasm/
├── package.json
├── tsconfig.json
├── tailwind.config.ts
├── next.config.js               # webpack mode with asyncWebAssembly experiment
└── README.md
```

## Core Systems (Research-Informed)

### 1. Energy Economy — Closed Budget System

**This is the single most important system for simulation stability** (Polyworld, Yaeger 1994).

Total world energy is conserved. Food spawning compensates for energy lost to death and metabolism:

```
energy_in_creatures = sum(creature.energy for all alive creatures)
energy_in_food = food_count * food_energy_value
deficit = target_total_energy - energy_in_creatures - energy_in_food
food_to_spawn = max(0, deficit / food_energy_value)
```

This naturally creates carrying capacity without a hard population cap. More creatures = less food spawns = more starvation = population decreases.

**Energy costs:**

| Action | Cost Formula | Rationale |
|--------|-------------|-----------|
| Existing (basal) | `base_cost * (1.0 + age / max_age)` | Senescence — creatures get less efficient with age |
| Movement | `speed² * mass * 0.15` where `mass = size²` | Quadratic speed cost prevents trivial speed maximization |
| Neural firing | `hidden_neuron_count * 0.01` (future) | Makes complex brains expensive (Polyworld insight) |
| Reproduction | 50% of parent energy transferred to child | Prevents rapid-fire splitting |

**The 3:1 Rule:** A fully-fed creature should survive ~3x longer than the average time to find one food pellet. This creates enough pressure to evolve food-seeking without instant starvation.

### 2. Physics — Momentum + Drag Model

Inertial physics produce richer evolved behaviors than instant movement (Framsticks, Komosinski 1999). Creatures must learn to brake, overshoot becomes a selection pressure, and ambush strategies emerge.

```
acceleration = (thrust_output * max_thrust) / mass
velocity += acceleration * dt
velocity *= (1.0 - drag_coefficient)    // linear drag
position += velocity * dt
// Toroidal wrapping
position.x = position.x.rem_euclid(world_width)
position.y = position.y.rem_euclid(world_height)
```

**Allometric scaling (size-speed tradeoffs):**
- `mass = size²` (2D area proxy)
- `max_thrust = size^1.5` (muscle scales with cross-section)
- Effective `max_speed ~ size^-0.5` (emerges from thrust/mass ratio)
- Small creatures: fast, agile, cheap to run. Large creatures: slow, expensive, but eat more efficiently.

**Turn range:** `[-PI/4, PI/4]` per tick — NOT `[-PI, PI]`. This prevents instant reversals, forces evolved locomotion strategy, and enables momentum-based hunting/fleeing behaviors.

### 3. Neural Network Brain

Fixed-topology feedforward network. Validated by biosim4 and evolv.io at this scale. Full NEAT is overkill for 1000+ creatures.

**Inputs (12):**

| # | Input | Range | Notes |
|---|-------|-------|-------|
| 0-1 | Nearest food: distance, angle_relative | [0,1], [-1,1] | Normalized by vision_range and PI |
| 2-3 | Food in left sector, food in right sector | [0,1] | Distance to nearest food in 60° left/right sectors (enables path planning, not just nearest-chasing) |
| 4-5 | Nearest creature: distance, angle_relative | [0,1], [-1,1] | Within vision cone only |
| 6-7 | Nearest creature: relative size, relative speed | [-1,1] | Enables threat assessment |
| 8 | Own energy (normalized) | [0,1] | energy / max_energy |
| 9 | Own speed (normalized) | [0,1] | current_speed / max_possible_speed |
| 10 | Own size (normalized) | [0,1] | Mapped to [0,1] range |
| 11 | Random noise | [-1,1] | Exploration, breaks symmetry |

**Hidden layer:** 8 neurons, tanh activation. User-tunable range 4-16.
- Below 4: cannot learn conditional strategies
- Above 16: mutation becomes inefficient (too many parameters for random search at this population size)

**Outputs (3):**

| # | Output | Mapping |
|---|--------|---------|
| 0 | Turn | tanh -> [-PI/4, PI/4] radians |
| 1 | Thrust | sigmoid -> [0, 1] * max_thrust |
| 2 | Reproduce | sigmoid -> threshold at 0.5 (AND energy sufficient AND maturation complete AND cooldown elapsed) |

**Genome size:** 12×8 + 8 (input-hidden weights + biases) + 8×3 + 3 (hidden-output weights + biases) = **131 parameters**.

**Weight clamping:** All weights clamped to `[-5.0, 5.0]`. Prevents tanh saturation drift where weights grow large and the network becomes insensitive to inputs.

**Vision cone:** 120° forward arc. Creatures cannot sense things behind them. This creates blind spots enabling ambush predation and evolving scanning behavior. Sensory inputs only populated for entities within the vision cone.

**Distance falloff:** `signal = 1.0 / (1.0 + distance / vision_range)` — nearby entities are much more salient than distant ones.

### 4. Genetics & Mutation

**Genome structure:**
```
[genome_version: u8] [trait_weights: f32 × 3] [brain_weights: f32 × 128] [mutation_rate: f32]
```

**Separate trait and brain mutation rates** — prevents trait optimization from dominating brain evolution:
- Trait weights: `mutation_strength * 0.5`
- Brain weights: `mutation_strength * 1.0`

**Mutation operators (per weight):**
- 90% Gaussian perturbation: `weight += N(0, stddev)`
- 10% Cauchy perturbation: `weight += Cauchy(0, stddev)` — heavy-tailed, escapes local optima
- Weight magnitude scaling: `stddev_effective = base_stddev * max(0.1, 1.0 - |weight| / 5.0)` — protects large weights from catastrophic disruption

**Heritable mutation rate** (key success factor from evolv.io):
- Each genome includes its own `mutation_rate` gene
- Mutated log-normally on reproduction: `rate' = rate * exp(tau * N(0,1))` where `tau ~ 1/sqrt(2 * genome_length)`
- Clamped to `[0.001, 0.5]`
- Lineages that find good solutions naturally evolve lower mutation rates to preserve them

**Physical trait mapping:**
- `speed = sigmoid(trait_weights[0]) * (max_speed - min_speed) + min_speed`
- `size = sigmoid(trait_weights[1]) * (max_size - min_size) + min_size`
- `vision_range = sigmoid(trait_weights[2]) * (max_vision - min_vision) + min_vision`

### 5. Speciation

**Genomic distance with separated components:**
```
d(a, b) = 0.3 * d_traits(a, b) + 0.7 * d_brain(a, b)
```
Where `d_traits` and `d_brain` are each normalized L1 distances over their respective genome regions. The 0.3/0.7 weighting prevents trait differences from dominating.

**Dynamic compatibility threshold** — targets 3-10 active species:
```
if num_species > target_max (10):
    threshold *= 1.05   // relax — merge similar species
if num_species < target_min (3):
    threshold *= 0.95   // tighten — encourage splitting
```
Adjusted every 100 ticks. Starting threshold: `2.0`.

**Species assignment:** Offspring joins parent species if `d(offspring, species_representative) < threshold`, else forks a new species with a new ID and assigned HSL color.

**Future upgrade path:** Behavioral speciation — compare network outputs on standardized test inputs instead of raw weights. Solves the hidden neuron permutation symmetry problem where functionally identical networks appear genetically distant. Cost: ~2ms for 1000 creatures.

### 6. Creature Lifecycle

| Phase | Duration | Rules |
|-------|----------|-------|
| **Birth** | tick 0 | Receives 50% of parent energy. Inherits mutated genome. Placed near parent. |
| **Maturation** | 40 ticks | Cannot reproduce. Must survive on inherited energy. Selects for viable offspring. |
| **Adult** | tick 40 to death | Can reproduce when: energy > threshold AND cooldown elapsed AND reproduce output > 0.5 |
| **Reproduction cooldown** | 120 ticks after each reproduction | Prevents chain-reaction reproduction explosions |
| **Senescence** | Gradual | Basal metabolic cost increases: `base_cost * (1.0 + age / max_age)`. No hard death from age — just increasing inefficiency. |
| **Death** | energy <= 0 | Backup hard cap at `max_lifespan` (3000 ticks) as safety valve |

### 7. Spatial Hash Grid (Roll-Your-Own)

Custom flat-array implementation (~100 lines of Rust). No external crate needed. 20-40% faster rebuild than SmallVec approach.

```rust
pub struct SpatialHash {
    cell_size: f32,
    inv_cell_size: f32,              // precomputed 1.0 / cell_size
    grid_width: usize,
    grid_height: usize,
    // Flat-array design: two parallel arrays instead of Vec<SmallVec<...>>
    count: Vec<u32>,                 // count[cell_index] = number of entities in cell
    entries: Vec<DenseKey>,          // packed contiguously per-cell after counting pass
    offsets: Vec<u32>,               // offsets[cell_index] = start index into entries[]
}

// Two-pass rebuild (cache-friendly, zero per-cell allocation):
// Pass 1: Zero counts, iterate creatures, increment count[cell(pos)]
// Pass 2: Prefix-sum counts → offsets. Iterate creatures again, place into entries[offset++]
```

- **Cell size** = 1× max vision range (not 2×). Reduces candidate set 20-30% vs 2× cells because query checks fewer neighbors.
- **Clear-and-rebuild every tick** — faster than incremental update for thousands of entities with the cache-friendly linear sweep
- **Shared between simulation and rendering** — same grid used for neighbor queries (sim) and viewport culling (render)
- Toroidal wrapping handled in cell index calculation
- Zero heap allocation after initial setup — all arrays pre-allocated to max capacity

### 8. World & Environment

**Food distribution — two-tier system:**
1. **Base layer:** Trickle of uniformly distributed food (prevents total starvation)
2. **Cluster layer:** Gaussian food clusters that appear, persist for N ticks, then shift location. Produces territorial behavior and explorer/exploiter specialization.

**Seasonal variation:**
```
food_rate_modifier = 1.0 + season_amplitude * sin(tick * 2π / season_period)
```
Default: `amplitude = 0.4`, `period = 1000 ticks`. Food rate varies 60%-140% of base. Produces boom-bust adaptation, energy storage strategies, and breaks evolutionary stagnation.

**Automated mild catastrophes:** Every 5000-10000 ticks, food halves for 200 ticks. Prevents permanent stagnation without manual intervention.

### 9. Render Buffer (Zero-Copy Pipeline)

Pre-allocated flat `Vec<f32>` in Rust with known capacity:

```rust
pub struct RenderBuffer {
    data: Vec<f32>,        // pre-allocated to max_creatures * FLOATS_PER_CREATURE
}

const FLOATS_PER_CREATURE: usize = 12;
// Per creature: [x, y, rotation, size, energy_norm, r, g, b, vx, vy, age_norm, species_id]
// vx, vy: velocity for client-side interpolation
// age_norm: for visual effects (maturation glow, senescence dimming)
// species_id: enables sort-by-species in Rust for pre-grouped path batching
```

**Critical WASM memory rule:** Any call into WASM that allocates can grow linear memory, invalidating all existing `ArrayBuffer` views. The Worker must re-acquire the `Float32Array` view after every `step()` call:

```typescript
// In Worker, after every step():
sim.step();
const ptr = sim.get_render_data_ptr();
const len = sim.get_render_data_len();
const view = new Float32Array(wasm.memory.buffer, ptr, len);
// Copy to transferable buffer for main thread
const buffer = new ArrayBuffer(view.byteLength);
new Float32Array(buffer).set(view);
postMessage({ type: 'frame', buffer, stats }, [buffer]);
```

**Double-buffer optimization:** Maintain two ArrayBuffers. Transfer one to main thread, receive the old one back. Avoids allocation each frame.

**Food render data:** Separate smaller buffer. Food changes less frequently — can update at lower rate.

## Data Flow (Refined)

```
User Input (controls/sliders)
    |
    v
Main Thread: React UI + Zustand store
    |
    v  Comlink RPC (start/stop/config/inspect)
Web Worker (owns WASM Simulation instance)
    |  - Loads .wasm from /public/wasm/ via init()
    |  - Runs fixed-timestep loop
    |  - Packs render buffer from WASM linear memory
    |
    v  postMessage: transferable ArrayBuffer (render) + JSON (stats, throttled 2-5Hz)
Main Thread receives data
    |
    ├──> useRef (frameRef) ──> requestAnimationFrame ──> Canvas 2D renderer
    |         (never enters React state — no re-renders)
    |
    └──> Zustand store (statsRef) ──> React charts (uPlot, throttled)
              (only updates at 2-5Hz, selective re-renders via subscribeWithSelector)
```

## Canvas Rendering Pipeline

**Batch rendering by species color** — the single biggest Canvas 2D optimization. Render buffer is pre-sorted by species_id in Rust, so JS iterates linearly without grouping:

```typescript
// Buffer arrives pre-sorted by species_id from Rust
let currentSpecies = -1;
let path: Path2D;
for (let i = 0; i < count; i++) {
    const offset = i * FLOATS_PER_CREATURE;
    const species = data[offset + 11];
    if (species !== currentSpecies) {
        if (path) ctx.fill(path);
        path = new Path2D();
        currentSpecies = species;
        ctx.fillStyle = speciesColors[species];
    }
    // Add creature to current path based on LOD tier
    drawCreatureToPath(path, data, offset, zoomLevel);
}
if (path) ctx.fill(path);
```

### Level of Detail (LOD) System

LOD tier determined by zoom level. Reduces draw complexity at zoomed-out views:

| Zoom Level | LOD | Rendering | Threshold |
|-----------|-----|-----------|-----------|
| < 0.3x | Dot | 1px filled circle | > 500 visible creatures |
| 0.3-0.7x | Triangle | Simple rotated triangle | 200-500 visible |
| 0.7-1.5x | Detailed | Triangle + energy bar + size variation | 50-200 visible |
| > 1.5x | Full | Triangle + eyes + vision cone (selected) + idle wobble | < 50 visible |

Eyes at close zoom are the "empathy generator" — seeing creatures look at food creates immediate engagement.

### Client-Side Movement Interpolation

The #1 visual polish priority. Smooths movement between simulation ticks:

```typescript
// Each frame, interpolate between last two sim positions
const alpha = (now - lastTickTime) / tickInterval;
const renderX = prevX + (currX - prevX) * alpha;
const renderY = prevY + (currY - prevY) * alpha;
```

Cost: negligible. Impact: transforms jerky motion into fluid movement.

### Visual Effects (All Client-Side, Zero WASM Changes)

| Effect | Implementation | Cost |
|--------|---------------|------|
| **Velocity elongation** | Stretch triangle along velocity vector: `length *= 1.0 + speed * 0.3` | Free (math only) |
| **Energy saturation** | HSL saturation scales with energy: `s = 40 + energy_norm * 60` | Free |
| **Birth pop-in** | Scale 0→1 over 10 frames with ease-out | Free |
| **Death particles** | 3-5 particles from pool, fade over 20 frames | <0.3ms for pool of 200 |
| **Idle wobble** | `rotation += sin(tick * 0.1) * 0.05` when speed < threshold | Free |
| **Selection highlight** | Pulsing ring + vision cone overlay on selected creature | Per-creature, not batched |
| **Trail rendering** | Offscreen canvas composited with `globalAlpha = 0.95` fade | One extra composite per frame |

**Never use `shadowBlur`** — 5-20x more expensive than manual glow. Use radial gradient or pre-rendered sprites instead.

Particle pool: pre-allocate 200 particle objects. On death event, grab from pool, animate client-side. Recycle when alpha reaches 0.

### Data Lens System

Toggle creature coloring mode with `C` key. Cycles through:

| Lens | Coloring | Purpose |
|------|----------|---------|
| Species (default) | Species HSL color | Track species visually |
| Energy | Red (low) → Green (high) gradient | See who's starving |
| Speed | Blue (slow) → Red (fast) | Visualize speed specialization |
| Age | White (young) → Dark (old) | See generational turnover |
| Mutation Rate | Cyan (low) → Magenta (high) | See evolutionary "temperature" |

Implemented by swapping the color lookup in the render loop — no buffer changes needed. Species colors stored in a JS map updated when new species form.

**Camera/viewport:**
- `ctx.setTransform(zoom, 0, 0, zoom, -camX * zoom, -camY * zoom)` before all drawing
- Viewport culling via spatial hash: only draw entities in visible cells
- Mouse wheel zoom (scale around cursor), mouse drag pan
- Reset transform for UI overlays

**DPI scaling:**
```typescript
const dpr = window.devicePixelRatio || 1;
canvas.width = displayWidth * dpr;
canvas.height = displayHeight * dpr;
canvas.style.width = displayWidth + 'px';
canvas.style.height = displayHeight + 'px';
ctx.scale(dpr, dpr);
```

**Canvas hints:** `getContext('2d', { desynchronized: true })` reduces latency by one frame (Chrome).

**Use integer coordinates** (`| 0`) for positions to avoid sub-pixel anti-aliasing cost.

## Performance Targets
- 1000+ creatures at 60fps (Rust/WASM handles simulation in Worker, JS only renders)
- Spatial hashing keeps neighbor queries O(1)
- Render data: direct WASM linear memory read -> copy to transferable ArrayBuffer -> main thread
- Double-buffered ArrayBuffer transfer (no allocation per frame)
- Canvas path batching: 1 draw call per species color group (not per creature)
- `slotmap` arena: O(1) insert/remove, no reallocation churn, generational key safety
- Stats throttled to 2-5Hz — React never re-renders at 60fps
- Pre-allocated render buffer in Rust (`Vec::with_capacity`) — no growth during simulation

## UI/UX Architecture

### Layout (Desktop — Primary Target)

```
┌─────────────────────────────────────────────────────────────────────┐
│  [Play/Pause] [Step] [Speed: ███░░ 3x] [Reset]  │  Tick: 45,230  │
│  [Data Lens: Species ▾]                          │  Gen: 142      │
├────────────────────────────────────────────┬──────────────────────────┤
│                                            │  Species Panel (300px)  │
│                                            │  ┌──────────────────┐  │
│           Simulation Canvas (70%)          │  │ Vorax (32%)      │  │
│                                            │  │ Celerith (28%)   │  │
│                                            │  │ Draconis (15%)   │  │
│                                            │  │ ...              │  │
│                                            │  └──────────────────┘  │
│                                            │  Selected Creature:    │
│                                            │  ┌──────────────────┐  │
│                                            │  │ Brain viz / Stats│  │
│              [Minimap 120x90]              │  │ Genome / Lineage │  │
│                                            │  └──────────────────┘  │
├────────────────────────────────────────────┴──────────────────────────┤
│  [Population ▾] [Traits ▾] [Event Log ▾]     (collapsible, 200px)  │
│  ┌─────────────────────────┐ ┌──────────────────────────────────┐  │
│  │ uPlot stacked area chart│ │ Event narration log              │  │
│  │ (population by species) │ │ "Vorax split into 2 subspecies" │  │
│  └─────────────────────────┘ └──────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
```

### First-Run Experience

- **No modals, no tutorials.** Simulation auto-starts on page load at 3x speed with default seed.
- Immediate "aha moment" within 10 seconds: creatures visibly moving toward food.
- Subtle tooltip on first hover over canvas: "Scroll to zoom, drag to pan, click to inspect"
- Bottom-left toast after 30s: "Press `?` for keyboard shortcuts"

### Keyboard Shortcuts (Progressive Disclosure)

| Key | Action | Discoverable Via |
|-----|--------|-----------------|
| `Space` | Play/pause | Button tooltip |
| `.` | Single step | Button tooltip |
| `1-5` | Speed presets | Speed slider tooltip |
| `C` | Cycle data lens | Lens dropdown |
| `G` | Toggle grid overlay | `?` menu |
| `I` | Toggle species inspector | `?` menu |
| `P` | Toggle parameter panel | `?` menu |
| `D` | Toggle diagnostics panel | `?` menu |
| `L` | Toggle event log | `?` menu |
| `M` | Toggle minimap | `?` menu |
| `F` | Follow selected creature | `?` menu |
| `Esc` | Deselect / close panel | Standard |
| `?` | Show all shortcuts | Toast hint |

### Event Narration System (Core Engagement Feature)

Auto-generated story of evolution. Research shows this is THE highest-engagement feature in evolution simulators.

```typescript
// Event types detected and narrated:
interface EvolutionEvent {
    tick: number;
    type: 'speciation' | 'extinction' | 'population_boom' | 'population_crash'
        | 'trait_record' | 'first_food_taxis' | 'catastrophe' | 'recovery'
        | 'niche_split' | 'dominant_shift';
    message: string;        // Human-readable narrative
    species_id?: number;
    data?: Record<string, number>;
}

// Example narration messages:
// "A new species emerged: Celerith — small, fast foragers splitting from Vorax"
// "Extinction: Draconis (lasted 340 generations) — outcompeted by Celerith"
// "Population boom! Vorax doubled in 50 ticks after discovering the eastern food cluster"
// "New record: fastest creature ever observed (speed: 7.8) in species Celerith"
// "Catastrophe struck — food halved for 200 ticks. 3 species are struggling."
// "Recovery: population rebounded to 80% of pre-catastrophe levels"
```

Events detected in Rust (cheap — checked every 100 ticks), narration text generated in TypeScript.

### Auto-Generated Species Names

Phonetic Latin-ish names generated from species genome signature:

```typescript
const PREFIXES = ['Vor', 'Cel', 'Dra', 'Nox', 'Lum', 'Ter', 'Pha', 'Kri', 'Sel', 'Mor'];
const MIDDLES = ['a', 'e', 'i', 'o', 'u', 'ae', 'ei', 'or', 'an', 'en'];
const SUFFIXES = ['x', 'th', 'is', 'on', 'us', 'ia', 'um', 'nis', 'rix', 'tor'];
// Hash species representative genome → deterministic name
// "Vorax", "Celerith", "Draconis", "Noxium", "Luminor"
```

### Species Panel (Right Sidebar)

- Sorted by population share (descending)
- Each entry: color swatch + name + population count + % + sparkline trend
- Click species → highlight all members on canvas, dim others
- Expand species → trait averages, age distribution, mutation rate

### Minimap (Bottom-Left of Canvas, 120x90px)

- Downsampled world view rendered at 2Hz (not every frame)
- Viewport rectangle overlay shows current camera position
- Click minimap to jump camera
- Color-coded by current data lens

### Creature Inspector (Right Sidebar, Below Species Panel)

Click any creature to select. Shows:
- Real-time brain visualization (inputs → hidden → outputs, connection weights as line thickness)
- Stats: energy, age, generation, children count, species
- Genome: trait weights, mutation rate
- "Follow" button: camera tracks this creature
- Lineage: parent chain (if tracked)

### Speed Control

- Default: 3x (research shows 1x feels too slow for initial engagement)
- Slider: 1x → 2x → 3x → 5x → 10x → 50x → Max
- Auto-suggest: after 60s at 1x, subtle toast "Tip: increase speed to see evolution faster"
- "Fast forward to next event" button — runs at max speed until next narration event fires, then pauses

### Intervention Tools (Milestone 10)

| Tool | Input | Effect | Why It's Fun |
|------|-------|--------|-------------|
| Meteor | Click location | Kill creatures in radius, destroy food | Punctuated equilibrium |
| Drought | Toggle | Halve food globally for N ticks | Stress-test species |
| Flood | Toggle | Double food globally for N ticks | Population boom → crash |
| Wall Drawing | Click-drag | Place impassable barriers | Geographic speciation (most satisfying) |
| Predator Mode | Click to place | Player-controlled predator eating creatures | Direct interaction |
| Remove walls | Click wall | Remove barrier segment | Reunite separated populations |

**Pre-intervention auto-save** enables "undo" — revert to before the meteor hit.

### Colorblind Accessibility

Use Wong/IBM colorblind-safe palette for species colors:
```typescript
const COLORBLIND_SAFE = [
    '#E69F00', // orange
    '#56B4E9', // sky blue
    '#009E73', // bluish green
    '#F0E442', // yellow
    '#0072B2', // blue
    '#D55E00', // vermillion
    '#CC79A7', // reddish purple
    '#000000', // black
];
// For >8 species: generate via golden angle hue rotation in L*a*b* space
```

### Mobile Considerations

- Canvas fills viewport, controls collapse to bottom sheet
- Pinch to zoom, two-finger drag to pan
- Simplified sidebar as swipe-in drawer
- Reduce default creature cap to 500

## Error Handling
- `console_error_panic_hook` in Rust `#[wasm_bindgen(start)]` — WASM panics produce readable stack traces
- React ErrorBoundary wraps simulation canvas — graceful fallback UI on crash
- Worker `onerror` handler surfaces WASM failures to UI as user-visible error state
- Config validation in Rust `Simulation::new()` — reject invalid seeds, out-of-range parameters with `Result<Simulation, String>`
- Accumulator cap in fixed-timestep loop: `max(accumulator, TICK_RATE * 5)` prevents spiral-of-death when frames are slow

## Default Parameter Values

Research-calibrated starting points:

```rust
pub struct SimConfig {
    // World
    world_width: f32,              // 800.0
    world_height: f32,             // 600.0
    target_total_energy: f32,      // 30000.0 (~350 creature carrying capacity)
    seed: u64,                     // user-provided or random

    // Food
    food_energy: f32,              // 15.0 (higher density, smaller pellets)
    food_cluster_count: u32,       // 3-5 gaussian clusters
    food_cluster_shift_period: u32,// 500 ticks
    season_period: u32,            // 1000 ticks
    season_amplitude: f32,         // 0.4 (food rate varies 60%-140%)

    // Creatures
    initial_population: u32,       // 100
    max_energy: f32,               // 200.0
    basal_cost: f32,               // 0.15 per tick (tuned for random-walker viability in M1)
    movement_cost_factor: f32,     // 0.02 (speed² * mass * factor, increase in M3 with brains)
    reproduction_threshold: f32,   // 140.0 (70% of max_energy)
    reproduction_energy_share: f32,// 0.5 (50% to offspring)
    maturation_period: u32,        // 40 ticks
    reproduction_cooldown: u32,    // 120 ticks
    max_lifespan: u32,             // 3000 ticks (safety cap)

    // Physics
    drag_coefficient: f32,         // 0.08
    max_turn_rate: f32,            // PI/4 radians per tick

    // Vision
    vision_cone_angle: f32,        // 2π/3 (120 degrees)
    min_vision_range: f32,         // 30.0
    max_vision_range: f32,         // 120.0

    // Traits
    min_speed: f32,                // 1.0
    max_speed: f32,                // 8.0
    min_size: f32,                 // 2.0
    max_size: f32,                 // 10.0

    // Brain
    hidden_neurons: u32,           // 8 (user range: 4-16)
    weight_clamp: f32,             // 5.0 (weights clamped to [-5, 5])

    // Mutation
    base_mutation_rate: f32,       // 0.08 per weight
    base_mutation_strength: f32,   // 0.2 (gaussian stddev)
    trait_mutation_scale: f32,     // 0.5 (traits mutate at half strength)
    cauchy_probability: f32,       // 0.1 (10% of mutations use Cauchy)
    mutation_rate_clamp: (f32, f32), // (0.001, 0.5) for heritable mutation rate

    // Speciation
    compatibility_threshold: f32,  // 2.0 (initial)
    target_species_min: u32,       // 3
    target_species_max: u32,       // 10
    threshold_adjust_rate: f32,    // 0.05 (5% per 100 ticks)
    trait_distance_weight: f32,    // 0.3
    brain_distance_weight: f32,    // 0.7

    // Catastrophes
    auto_catastrophe_interval: u32,// 7500 ticks
    catastrophe_duration: u32,     // 200 ticks
    catastrophe_food_multiplier: f32, // 0.5
}
```

## Emergent Behavior Timeline (Validation Benchmarks)

| Generation | Ticks | Expected Behavior | Red Flag If Not Seen |
|-----------|-------|-------------------|---------------------|
| 1-10 | 0-2000 | Random walk. Differential survival by luck. Population crash from initial, then recovery. | Population hits 0 (energy balance wrong) |
| 10-50 | 2000-15000 | **Food taxis** — creatures turn toward food. Average food-distance decreasing over generations. | No food taxis by gen 50 (mutation rate too low or inputs not normalized) |
| 50-200 | 15000-80000 | Speed/size specialization. 3-8 species visible. Improved foraging efficiency. | All creatures identical (compatibility threshold too high) |
| 200-1000 | 80K-500K | Energy management (resting when full). Reproductive timing. Seasonal adaptation. | One species dominates permanently (no environmental variation) |
| 1000+ | 500K+ | Coevolutionary dynamics. Complex foraging patterns. Niche partitioning. | Spinning in circles (turn range too wide) |

**The Food Taxis Test:** Run 50 generations. Plot average distance between creatures and nearest food. If not decreasing, debug the selection pressure and input normalization.

## Playability & Engagement

### Design Philosophy: "The UI is the Narrator"

The simulation generates data. The UI tells the story. Every chart, label, and event log entry should help the user understand *what is happening evolutionarily*, not just what the numbers are.

### Engagement Loop

```
Observe (watch creatures) → Notice (species diverge) → Understand (event log explains why)
    → Intervene (drop meteor) → Observe consequences → Share (seed URL / screenshot)
```

### "Fast Forward to Next Event" Button

Eliminates the #1 cause of disengagement: stagnation. Runs simulation at max speed until the next narration event fires, then auto-pauses. Users skip boring stretches without missing key moments.

### Species Chronicle (Summary View)

Auto-generated narrative summary accessible from species panel:

```
SPECIES CHRONICLE: Vorax
━━━━━━━━━━━━━━━━━━━━━━━
Founded: Generation 12 (split from Proto-species)
Peak population: 234 (Gen 89)
Key adaptations: High speed (avg 6.2), small body (avg 3.1)
Survived: 2 catastrophes, 1 drought
Descendant species: Celerith (Gen 67), Noxium (Gen 134)
Current status: Dominant (38% of population)
Strategy: Fast forager, cluster-hopping
```

### "What If" Experiment Mode (Milestone 10+)

Save → intervene → compare. Transforms passive watching into active science.

1. Auto-save at intervention point
2. User applies intervention (meteor, wall, drought)
3. Run for N ticks
4. Offer: "Compare with pre-intervention timeline?"
5. Split-screen or overlay showing divergence in population/traits

### Achievement Toasts (Non-Blocking)

Subtle corner notifications for evolutionary milestones:
- "First food taxis! Creatures are learning to find food." (Gen ~20)
- "Speciation! The first new species has emerged." (Gen ~50)
- "Adaptive radiation — 5+ species coexisting." (Gen ~200)
- "Survived catastrophe — life finds a way." (after recovery)
- "Speed demon — a creature reached max speed." (trait record)

No gamification points or unlocks. Just narrative milestones that teach the user what to watch for.

### Competitive Analysis Insights Applied

| Competitor | What Works | Applied Here |
|-----------|-----------|-------------|
| The Bibites | Narration + story creates attachment | Event narration system |
| Lenia | Beautiful but not interactive | Wall drawing + interventions |
| evolv.io | Immediate "aha moment" | Auto-start at 3x, food taxis within 10s |
| SALRE | Deep but steep learning curve | Progressive disclosure via keyboard |

## Milestones

| # | Milestone | Deliverable | Validation |
|---|-----------|-------------|------------|
| 1 | Rust foundation | DenseSlotMap creatures, seeded SmallRng, flat-array spatial hash, closed energy budget, momentum physics, WASM builds with panic hook, tick phase profiling. Creatures move randomly, eat food, die, reproduce (with maturation + cooldown). No brain. | Determinism test passes. Population stabilizes at carrying capacity. Phase timings visible. |
| 2 | Web Worker bridge | WASM in Worker via `--target web`. Comlink RPC for commands. Transferable ArrayBuffer for render data (12 floats/creature). Double-buffered. Watchdog heartbeat. | Round-trip latency <1ms. No main thread jank. Watchdog recovers from simulated hang. |
| 3 | Neural network brain | 12-input, 8-hidden, 3-output feedforward NN with fixed-size weight arrays. Vision cone with dot/cross cone check. Squared distances. Distance falloff. Weight clamping. Fused vision+NN loop. | Food taxis emerges within 50 generations. NN + vision < 75% of tick time. |
| 4 | Mutation + speciation | Gaussian+Cauchy mutation. Heritable mutation rate. Separate trait/brain rates. Dynamic speciation threshold. Auto-generated species names. | 3-10 species form. Trait distributions diverge. Species have readable names. |
| 5 | Environment | Seasonal food variation. Clustered food. Automated mild catastrophes. | Population oscillates with seasons. Recovery after catastrophes. |
| 6 | Canvas renderer | Path-batched Canvas 2D (pre-sorted by species). LOD system (4 tiers). DPI scaling. Camera pan/zoom. Viewport culling. Movement interpolation. Velocity elongation. Energy saturation. | 1000 creatures at 60fps. Smooth zoom/pan. Fluid movement between ticks. |
| 7 | UI shell + controls | Dark theme layout (70% canvas / 30% sidebar). Play/pause/step/reset. Speed slider (default 3x). Data lens toggle. Keyboard shortcuts. Minimap. Auto-start on load. | Config changes take effect within 1 frame. No modals on first load. |
| 8 | Stats + narration | uPlot stacked area population chart. Trait distribution chart. Event narration log with auto-detected events. Species panel with sparklines. Achievement toasts. Throttled to 5Hz. | Charts update smoothly. Narration fires on speciation/extinction/records. |
| 9 | Creature inspector | Click to select. Brain visualization. Species chronicle. Genome/stats panel. Follow mode. Birth pop-in + death particle effects. | Selection works at all zoom levels. Chronicle generates readable narrative. |
| 10 | Interventions | Meteor, drought, flood, wall drawing, predator mode. Pre-intervention auto-save. "Fast forward to next event" button. | Mass extinction followed by adaptive radiation. Wall creates geographic speciation. |
| 11 | Save/load + presets | Worker-based IndexedDB with write-ahead rotation. Genome-versioned binary export/import. Preset configs. Shareable seed URLs. Safari file-download fallback. | Load a saved sim, verify identical continuation. Safari persistence works. |
| 12 | Polish + deploy | Colorblind-safe palette. Mobile responsive layout. "What If" experiment mode. Landing page, OG image, Vercel deploy, README, perf benchmarks. | Lighthouse score > 90. Colorblind-accessible. Mobile usable at 500 creatures. |

## Key Design Decisions

1. **Rust for simulation, not rendering** — Canvas 2D handles 1000-3000 entities fine. Rust handles neural nets, physics, spatial hashing. JS handles pixels.
2. **Web Worker from day one** — Simulation never blocks the main thread. Worker owns WASM; main thread only renders and handles UI.
3. **Zero-copy render pipeline** — Render data in WASM linear memory as flat f32 buffer. JS reads via pointer, copies to transferable ArrayBuffer for Worker->main transfer. Re-acquire view after every `step()`.
4. **Deterministic seeded RNG** — Single `SmallRng` seeded from config. All randomness flows through it. Enables reproducible runs and shareable seeds.
5. **`slotmap` for creatures** — Generational keys prevent dangling references. O(1) insert/remove. Dense storage for cache-friendly iteration. Replaces `generational-arena` (less maintained).
6. **Closed energy budget** — Total world energy conserved. Food spawning compensates for losses. Naturally creates carrying capacity without hard caps. The most critical stability mechanism (Polyworld).
7. **Momentum + drag physics** — Quadratic speed cost, inertial movement, limited turn rate. Produces richer evolved locomotion than instant point-and-move.
8. **Fixed-topology NN (not NEAT)** — Validated at this scale by biosim4/evolv.io. NEAT limits population to hundreds and adds complexity without proportional benefit for 2D foraging.
9. **Separated trait/brain mutation** — Trait genes mutate at 0.5x brain rate. Prevents body optimization from swamping brain evolution. Genomic distance weighted 0.3 traits / 0.7 brain.
10. **Heritable mutation rate** — Self-adaptive evolution strategy. Lineages naturally tune their own exploration rate. Key success factor from evolv.io.
11. **Dynamic speciation threshold** — Auto-adjusts to maintain 3-10 species. Prevents species explosion (hundreds of tiny groups) or collapse (one megaspecies).
12. **Seasonal environment** — Sinusoidal food variation + automated mild catastrophes. Breaks evolutionary stagnation without manual intervention.
13. **Maturation + cooldown** — 40-tick maturation, 120-tick reproduction cooldown. Prevents chain-reaction reproduction and selects for viable offspring.
14. **Vision cone (120°)** — Creates blind spots, evolves scanning behavior, enables ambush strategies. More interesting than omnidirectional sensing.
15. **Zustand + useRef rendering** — Stats throttled to 2-5Hz in Zustand store. Render data in useRef, read by rAF loop. React never re-renders at 60fps.
16. **uPlot for live charts** — Canvas-based, 30KB, handles streaming time-series data. Recharts (SVG) cannot keep up with real-time updates.
17. **Toroidal world** — No edge effects. Creatures exiting one side appear on the other.
18. **Genome versioning** — `genome_version: u8` header enables future structural mutations with save migration.
19. **Auto-start, no modals** — Simulation begins immediately on page load at 3x speed. Progressive disclosure via keyboard shortcuts. Immediate "aha moment" within 10 seconds.
20. **Event narration as core feature** — Auto-generated story of evolution is the #1 engagement driver. Speciation, extinction, records, catastrophes all narrated in human-readable text.
21. **Movement interpolation** — Client-side interpolation between sim ticks. Single highest-impact visual polish item. Zero WASM cost.
22. **LOD rendering** — 4 detail tiers based on zoom level. Dots at far zoom, eyes at close zoom. Enables both overview and intimate observation.
23. **Data lens system** — Toggle creature coloring between species/energy/speed/age/mutation-rate. Reveals hidden patterns without cluttering the default view.
24. **Flat-array spatial hash with 1x cell size** — Two-pass counting design, zero per-cell allocation. 20-40% faster than SmallVec. 1x cell size reduces candidates 20-30% vs 2x.
25. **Fixed-size NN weight arrays** — Compiler unrolls inner loops. 20-40% faster than dynamic Vec for forward pass.
26. **Instrument from day one** — Tick phase profiling built into the first Rust build. All optimization decisions measurement-driven.
27. **Species names, not numbers** — Auto-generated phonetic Latin names create attachment. "Vorax went extinct" hits harder than "Species #7 went extinct."
28. **Wall drawing as geographic speciation tool** — Most satisfying intervention. Creates isolated populations that diverge independently.

## Build Pipeline

```bash
# Development (two terminals)
# Terminal 1: Watch Rust changes, auto-rebuild WASM
cargo watch -w crates/simulation/src -s "scripts/build-wasm.sh"

# Terminal 2: Next.js dev server (webpack mode for Worker bundling)
pnpm dev  # → next dev --webpack

# scripts/build-wasm.sh
cd crates/simulation
wasm-pack build --target web --out-dir pkg
wasm-opt pkg/simulation_bg.wasm -O3 -o pkg/simulation_bg.wasm  # release only
cp pkg/simulation_bg.wasm ../../public/wasm/
cp pkg/simulation.js ../../src/workers/wasm/
cp pkg/simulation.d.ts ../../src/workers/wasm/
```

## Optimization Strategy

### Day-One Instrumentation

Instrument phase timings from the first Rust build. All optimization decisions must be measurement-driven.

```rust
pub struct TickProfile {
    pub spatial_hash_us: u32,
    pub vision_us: u32,
    pub neural_net_us: u32,
    pub physics_us: u32,
    pub reproduction_us: u32,
    pub energy_us: u32,
    pub total_us: u32,
    pub creature_count: u32,
}
// Exposed to JS via render buffer footer. Displayed in diagnostics panel.
```

**Expected phase distribution (1000 creatures):**
- Vision queries: 30-45% of tick time
- NN forward pass: 20-30%
- Spatial hash rebuild: 5-10%
- Physics integration: 5-10%
- Reproduction + energy: 5-10%
- Everything else: <5%

Vision + NN = 50-75% of tick time. Optimize these first.

### Tier 1 Optimizations (Implement from Day One)

These are architectural choices, not premature optimization:

| Optimization | Impact | Notes |
|-------------|--------|-------|
| `DenseSlotMap` (not `SlotMap`) | Better iteration cache locality | Already in tech stack |
| Flat-array spatial hash | 20-40% faster rebuild | Already in spatial hash spec |
| Cell size = 1× max vision range | 20-30% fewer candidates per query | Already in spatial hash spec |
| Squared distances (no `sqrt`) | ~15% on vision queries | Compare `d² < r²` instead of `d < r` |
| Dot/cross product cone check | Faster than `atan2` for 120° cone | `dot > 0.5 * len` for 120° |
| Fused vision + NN loop | 5-10% from reduced iteration overhead | Single pass: gather inputs → forward pass → apply outputs |
| Fixed-size NN weight arrays | 20-40% on NN forward pass | `[f32; 96]` + `[f32; 24]` + `[f32; 8]` + `[f32; 3]` — compiler unrolls loops |
| `#[inline(always)]` on hot functions | Eliminates call overhead | `forward_pass`, `query_neighbors`, `apply_physics`, `distance_squared`, `in_vision_cone` |
| Sort render buffer by species in Rust | Pre-grouped for JS path batching | Already in render buffer spec |

### Tier 2 Optimizations (Apply When Profiling Shows Need)

| Optimization | Impact | When to Apply |
|-------------|--------|--------------|
| Split brain weights from Creature struct | 15-25% on non-NN phases | When physics/energy iteration is hot |
| Viewport culling in Rust | Reduces render buffer size | When >2000 creatures cause render overhead |
| WebGL instanced rendering | Needed for >3000 creatures | Thin wrapper: ~200 LOC, one instanced draw call per species |
| SIMD via `std::simd` (nightly) | 2-4x on NN forward pass | When NN is confirmed bottleneck and WASM SIMD is stable |

### Scaling Expectations

| Creatures | Expected Tick Time (after Tier 1) | Notes |
|-----------|----------------------------------|-------|
| 500 | ~2ms | Comfortable 60fps at any speed |
| 1,000 | ~3-5ms | Target sweet spot |
| 2,000 | ~8-12ms | Still 60fps with time-budgeted stepping |
| 3,000 | ~15-20ms | Needs WebGL for rendering, sim still fits in 16ms budget at 1x |
| 5,000+ | ~30-40ms | Requires Tier 2 opts + reduced tick rate |

## Testing Strategy

### Simulation Tests (Rust)
- **Determinism:** Run 1000 ticks with same seed twice, assert byte-identical final state
- **Cross-browser determinism:** Verify polynomial `tanh` produces identical results to reference values
- **Stability:** Run 5000 ticks, assert 10 < population < 1000 (no explosion or collapse)
- **Food taxis:** Run 50 generations, assert avg creature-to-food distance is decreasing
- **Speciation:** Run 200 generations, assert 2+ species exist
- **NaN resilience:** Inject NaN into creature fields, verify killed without propagation
- **Energy conservation:** Run 10000 ticks, assert energy drift < 1%
- **Unit tests:** Brain forward pass, mutation operators, spatial hash queries, energy budget, genomic distance, species assignment, physics integration

### Security Tests (Rust)
- **Config boundary:** Test every parameter at min, max, below-min, above-max, NaN, Infinity, negative
- **Cross-parameter:** Verify rejection of configs where `reproduction_threshold < food_energy * 2`
- **Save fuzzing:** Feed random bytes to deserializer, verify no panic (always `Err`)
- **Save bomb:** Crafted save claiming 10M creatures, verify rejection by `bincode::with_limit`
- **Decompression bomb:** High-ratio compressed payload, verify rejection
- **Population cap:** Force reproduction at MAX_CREATURES, verify blocked
- **Memory budget:** Config that would exceed 512MB estimate, verify rejected before allocation
- **Genome version:** Unknown version in save file, verify clean rejection

### Integration Tests (TypeScript)
- **Worker lifecycle:** Verify Comlink RPC protocol, transferable buffer round-trip, double-buffer swap
- **Watchdog:** Simulate hung Worker (mock), verify terminate + respawn + save recovery
- **URL params:** Verify allowlist filtering, extreme value rejection, XSS payload sanitization
- **State machine:** Verify all valid transitions succeed, all invalid transitions are rejected
- **IndexedDB failure:** Simulate quota exceeded, verify graceful fallback messaging

### E2E Tests (Playwright)
- **Visual regression:** Screenshot tests for Canvas rendering
- **Mobile viewport:** Verify adaptive quality tiers trigger on constrained viewport
- **Save roundtrip:** Save → reload → load → verify simulation continues identically

### Performance (Criterion)
- `step()` with 1000 creatures — target <8ms
- `step()` with 5000 creatures — target <40ms
- Spatial hash rebuild — target <1ms for 5000 entities
- Render buffer pack — target <0.5ms for 5000 creatures

## Security Architecture

Full audit in `SECURITY_AUDIT.md`. Key requirements integrated below.

### Trust Boundaries

```
UNTRUSTED INPUTS (validate before use):
├── URL query parameters (seed, config)        → allowlisted keys, typed parsing, range validation
├── Uploaded binary save files                  → size limit, checksum, bincode::with_limit, field validation
├── UI slider values (DevTools-modifiable)      → validated in BOTH TypeScript and Rust
└── IndexedDB loaded saves (potentially corrupt)→ same validation as uploaded files

TRUSTED INTERNALS:
├── WASM linear memory (sandboxed by browser engine)
├── Worker thread (isolated from DOM, cookies, localStorage)
└── Structured clone via postMessage (no shared state unless SharedArrayBuffer)
```

### Config Validation (Mandatory — The #1 Security Requirement)

Every parameter validated in Rust `Simulation::new()` and `update_config()`. Return `Err(String)` for invalid configs — reject, do not silently clamp.

```rust
// Enforced validation ranges (Rust is the trust boundary)
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
reproduction_energy_share:[0.1, 0.9]        // >= 0.3 enforced to prevent rapid-fire reproduction
maturation_period:        [1, 1000]
reproduction_cooldown:    [1, 10000]         // must be >= maturation_period / 2
max_lifespan:             [100, 100000]
drag_coefficient:         [0.001, 1.0]
max_turn_rate:            [0.01, PI]
base_mutation_rate:       [0.0, 1.0]
base_mutation_strength:   [0.0, 5.0]
season_amplitude:         [0.0, 0.9]
seed:                     [0, u64::MAX]     // all u64 valid
```

**Cross-parameter validation:**
- `reproduction_threshold > food_energy * 2` (creatures must forage)
- `reproduction_cooldown >= maturation_period / 2` (prevents runaway reproduction)

**Memory budget gate before allocation:**
```rust
let estimated_bytes = initial_population * (CREATURE_SIZE + hidden_neurons * 12 * 4)
    + (world_width / cell_size) as usize * (world_height / cell_size) as usize * CELL_SIZE
    + initial_population * FLOATS_PER_CREATURE * 4;
if estimated_bytes > 512_000_000 { return Err("Config exceeds memory budget"); }
```

TypeScript mirrors these validations for immediate UI feedback, but Rust is authoritative.

### Save/Load Security

```
Save format:
[magic: 4 bytes "DWSB"]
[format_version: u16]
[checksum: u32 CRC32 of everything after this field]
[compressed_payload: deflate via flate2]
```

**Deserialization defense-in-depth:**
1. **File size limit:** Reject uploads > 50MB in UI before sending to Worker
2. **Decompression cap:** `decoder.take(MAX_DECOMPRESSED_SIZE)` — reject if ratio > 100:1
3. **Bincode limit:** `bincode::Options::with_limit(MAX_SAVE_SIZE_BYTES)`
4. **Checksum verification:** CRC32 check before deserialization
5. **Post-deserialization validation:** Every field validated as if it were untrusted config input
   - All f32 values: `f.is_finite()` (rejects NaN and Infinity)
   - All positions within world bounds
   - All energies >= 0
   - Brain weight array lengths match `hidden_neurons`
   - `genome_version` is a known supported version (unknown → reject, never guess)
   - Population count within bounds
   - SlotMap keys consistent (no dangling references)

### URL Parameter Security

```typescript
// Allowlisted keys only — ignore all others
const ALLOWED_KEYS = new Set(['seed', 'world_width', 'world_height', ...]);
const params = new URLSearchParams(window.location.search);
// Use URLSearchParams (immune to prototype pollution), not qs or manual parsing
// First-value wins for duplicate keys
// Apply identical validation ranges as Rust config
// Never render URL values as raw HTML (React JSX escaping handles this)
// Limit total URL length to 2048 characters
```

### Security Headers (next.config.js)

```
Content-Security-Policy:
  default-src 'self';
  script-src 'self' 'wasm-unsafe-eval';    // minimal WASM permission (not 'unsafe-eval')
  worker-src 'self' blob:;
  style-src 'self' 'unsafe-inline';
  img-src 'self' data: blob:;
  connect-src 'self';
  font-src 'self';
  object-src 'none';
  base-uri 'self';
  form-action 'self';
  frame-ancestors 'none';

Cross-Origin-Opener-Policy: same-origin
Cross-Origin-Embedder-Policy: require-corp
X-Content-Type-Options: nosniff
X-Frame-Options: DENY
Referrer-Policy: strict-origin-when-cross-origin
Permissions-Policy: camera=(), microphone=(), geolocation=()
Strict-Transport-Security: max-age=63072000; includeSubDomains; preload
```

### Supply Chain

- `cargo audit` in CI on every build (RustSec Advisory Database)
- `pnpm audit` in CI on every build
- `smallvec` >= 1.0 mandatory (CVE-2019-15551 in 0.x)
- Pin exact versions in `Cargo.lock` and `pnpm-lock.yaml`, both committed
- Minimal `#[wasm_bindgen]` export surface — do not expose arbitrary memory access
- `wasm-opt` installed from official Binaryen releases only

## Failsafe Architecture

Full specification in `FAILSAFE-ARCHITECTURE.md`. Key mechanisms integrated below.

### Simulation Lifecycle State Machine

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

All transitions validated by a single `transition(from, to)` function. Invalid transitions logged and rejected.

### Population Safeguards

| Threshold | Action |
|-----------|--------|
| 0 creatures | Pause simulation. Modal: "All creatures died." Offer reseed / revert / reset. |
| 3,000 creatures | Yellow warning. Increase basal metabolic cost by 50% temporarily. |
| 5,000 creatures | Orange warning. Block reproduction until population drops below 4,000. |
| 10,000 creatures (hard cap in Rust) | Safety valve. Skip ALL reproduction. Cannot be configured away. |

Never randomly cull — it destroys evolutionary progress.

### NaN Guard (Per-Creature, Every Tick)

```rust
// After neural net forward pass and physics integration:
if position.x.is_nan() || position.y.is_nan() || energy.is_nan() || velocity.x.is_nan() {
    kill_creature(id);  // remove before neighbor queries prevent propagation
    nan_deaths += 1;
}
if nan_deaths > 10 {  // systemic bug, not one-off
    enter_error_state();
}
```

Cost: negligible (branch prediction — NaN is extremely rare). Prevention: weight clamping to [-5,5], bounded distance formula `1/(1+d/r)`, energy floored at 0.

### Energy Budget Audit (Every 100 Ticks)

```rust
let actual = creatures.iter().map(|c| c.energy).sum::<f32>() + food_count * food_energy;
let drift_ratio = (actual - target_total_energy).abs() / target_total_energy;
if drift_ratio > 0.02 {
    // Auto-correct via food spawning adjustment (inherently self-correcting)
    log_warn!("Energy drift {:.1}%, correcting", drift_ratio * 100.0);
}
```

### Worker Watchdog (Main Thread)

```typescript
let lastHeartbeat = Date.now();
worker.onmessage = (e) => { lastHeartbeat = Date.now(); /* ... */ };
setInterval(() => {
    if (Date.now() - lastHeartbeat > 5000) {
        worker.terminate();
        // Spawn new Worker, load last auto-save
        // If auto-save also hangs → try last_known_good → oldest save → fresh reset
    }
}, 1000);
```

### Time-Budgeted Stepping (Prevents Worker Hang at High Speed)

```typescript
// In Worker — NOT a fixed step_n(speed)
const TICK_BUDGET_MS = 14; // leave 2ms headroom
let ticks_done = 0;
const start = performance.now();
while (ticks_done < requested_ticks && (performance.now() - start) < TICK_BUDGET_MS) {
    sim.step();
    ticks_done++;
}
// Report actual speed to UI: "Speed: 20/50 (limited by device)"
```

### Save System (Write-Ahead Rotation)

```
Slot A: [valid save from 5 min ago]   ← "latest" pointer
Slot B: [writing new save...]          ← crash here = Slot A still valid
Slot C: [older save]
```

- Never overwrite current save. Write to next slot, update "latest" pointer only after IDB transaction commits.
- 3 rotating auto-save slots + "last_known_good" (only updated after 60s of stable simulation)
- Pre-intervention auto-save before meteor/drought/flood (enables undo)
- `navigator.storage.persist()` to prevent browser eviction
- Safari 7-day IDB eviction warning: offer "Download save file" as primary persistence on Safari

### Canvas Context Recovery

```typescript
canvas.addEventListener('contextlost', (e) => {
    e.preventDefault(); // signal we want restoration
    contextIsLost = true;
    showOverlay("Display temporarily unavailable. Simulation continues.");
});
canvas.addEventListener('contextrestored', () => {
    ctx = canvas.getContext('2d', { desynchronized: true });
    applyDPIScaling(ctx);
    contextIsLost = false;
});
// Simulation continues in Worker regardless — only rendering is affected
```

### Adaptive Quality Tiers

| Tier | FPS Threshold | Actions |
|------|--------------|---------|
| Full | >= 50 fps | All features |
| Reduced | 30-50 fps | Simplify food rendering, stats to 1Hz, circles instead of triangles |
| Minimal | 15-30 fps | DPR = 1.0, render every 2nd frame, reduce creature cap to 2,000 |
| Emergency | < 15 fps | Pause rendering. Stats-only view. Prompt "Reduce creature count?" |

### Graceful Degradation

| Missing Feature | Response |
|----------------|----------|
| No Web Worker | Hard block: "Requires Web Workers. Use a modern browser." |
| No WASM | Hard block: "Requires WebAssembly. Use a modern browser." |
| IndexedDB unavailable (private browsing) | Disable auto-save. Banner: "Saves unavailable in private browsing." Offer file download. |
| iOS Safari (< 256MB WASM practical limit) | Detect mobile, reduce default creature cap to 1,000 |
| Tab backgrounded | Safari suspends Workers after ~30s. Listen `visibilitychange`, checkpoint state. Throttle sim in background. |

## Cross-Browser Compatibility

### Determinism Guarantees

| Scope | Guaranteed? | Notes |
|-------|-------------|-------|
| Same browser, same machine, same seed | **Yes** | Core determinism guarantee |
| Same browser, different machine | **Mostly** | FMA behavior may differ between ARM and x86 |
| Different browsers, same seed | **No** | `tanh`/`sin`/`cos` use host libm which differs between V8/SpiderMonkey/JSC |

**Mitigation:** Polynomial `tanh` approximation (Pade: `x*(27+x²)/(27+9x²)`, max error ~0.004) uses only `+,-,*,/` which ARE bit-identical across all WASM engines. This makes the neural network forward pass deterministic. `sin`/`cos` only used for seasonal variation (food rates) — tiny differences acceptable.

### Browser-Specific Constraints

| Browser | Constraint | Mitigation |
|---------|-----------|------------|
| iOS Safari | ~256MB WASM memory before tab kill (no warning) | Cap WASM memory. Detect iOS, reduce creature limit. |
| iOS Safari | Canvas max ~16.7M pixels on older devices | Cap canvas backing store. Use viewport rendering. |
| Safari (all) | 7-day IndexedDB eviction if user doesn't return | Warn users. Primary persistence = file download. `navigator.storage.persist()`. |
| Safari (all) | Workers suspended after ~30s in background | `visibilitychange` checkpoint. Accept pause. |
| Safari (all) | `performance.now()` only 1ms precision without COOP/COEP | Set COOP/COEP headers (already in security headers). |
| Safari (all) | WASM needs correct `application/wasm` MIME type (strict) | Verify Vercel serves `.wasm` correctly. |
| Firefox | `performance.now()` 20μs precision without COOP/COEP | Set COOP/COEP headers. |
| All mobile | Thermal throttling after 30-60s sustained WASM compute | Adaptive quality tiers. Monitor frame times. |
| All browsers | `memory.grow()` invalidates all `ArrayBuffer` views | Re-acquire `Float32Array` view after every `step()` call. |

### Minimum Browser Versions

| Browser | Minimum | Reason |
|---------|---------|--------|
| Chrome | 90+ | Module Workers, WASM stable, Path2D |
| Firefox | 114+ | Module Workers support |
| Safari | 16.4+ | WASM exceptions, stable IndexedDB in private browsing |
| Edge | 90+ | Chromium-based, same as Chrome |
| iOS Safari | 16.4+ | Same as Safari |

### WASM Binary Configuration

```toml
# Cargo.toml
[profile.release]
panic = "abort"        # smaller binary, no unwind tables
opt-level = 3          # max optimization
lto = true             # link-time optimization
codegen-units = 1      # max optimization (slower compile)

[profile.release.package.simulation]
# Set memory limits
# Initial: 16MB, Maximum: 256MB
```

## Diagnostics Panel

Toggleable panel (keyboard shortcut: `D`). All values updated at 2Hz.

```
FPS: 60 | Tick: 8.2ms | Creatures: 847 | Food: 312
Memory: 24 MB | Speed: 5x (actual: 5x) | Gen: 142
Energy drift: 0.1% | NaN deaths: 0 | State: Running
Last save: 12s ago | Quality: Full
```

Color-coded: green = healthy, yellow = warning, red = critical.

**Error event log** (circular buffer of last 100 events):
```
[tick 45230] WARN: Energy drift 1.2%, correcting
[tick 45180] WARN: Population soft cap reached (3,012)
[tick 44900] INFO: Auto-save completed (245 KB)
[tick 44100] ERROR: NaN detected in creature #2847, killed
```

Exportable as text for bug reports.

## References
- Yaeger, L. (1994). "Computational Genetics, Physiology, Metabolism, Neural Systems, Learning, Vision, and Behavior; or PolyWorld: Life in a New Context." — Energy budget, neural firing cost
- Stanley, K. & Miikkulainen, R. (2002). "Evolving Neural Networks through Augmenting Topologies" (NEAT) — Speciation, compatibility distance
- Komosinski, M. & Ulatowski, S. (1999). "Framsticks" — Momentum-based physics for evolved locomotion
- Sims, K. (1994). "Evolving 3D Morphology and Behavior by Competition" — Co-evolution, sensory-motor evolution
- Beer, R.D. (2003). "The Dynamics of Active Categorical Perception in an Evolved Model Agent" — Minimal networks for complex behavior
- davidrmiller/biosim4 — Fixed-topology validation at scale
- carykh/evolv.io — Heritable mutation rates, natural selection dynamics

## Companion Documents
- `SECURITY_AUDIT.md` — Full threat model with 30+ findings rated by severity
- `FAILSAFE-ARCHITECTURE.md` — Complete failure mode analysis with detection, prevention, recovery, and user communication for every failure mode
