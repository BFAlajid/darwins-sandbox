# Darwin's Sandbox

Real-time evolution simulator where creatures with neural network brains compete for food, reproduce with mutations, and undergo natural selection — all running in your browser.

Built with Rust/WASM for the simulation engine and Next.js for the frontend.

## Features

- **Closed energy budget** — total world energy is conserved, naturally creating carrying capacity
- **Momentum physics** — creatures have inertia, drag, and limited turn rates
- **Neural network brains** — 12-input, 8-hidden, 3-output feedforward networks (upcoming)
- **Genetic mutation** — Gaussian + Cauchy operators with heritable mutation rates (upcoming)
- **Speciation** — genomic distance clustering with auto-generated species names (upcoming)
- **Canvas rendering** — LOD system, species color batching, pan/zoom camera
- **Web Worker isolation** — simulation never blocks the UI thread
- **Time-budgeted stepping** — maintains 60fps regardless of simulation speed

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Simulation | Rust → WASM (156KB binary) |
| Entity Storage | `slotmap` (DenseSlotMap) |
| Spatial Index | Custom flat-array spatial hash |
| Frontend | Next.js 14, TypeScript, Tailwind CSS |
| State | Zustand (5Hz throttled stats) |
| Rendering | HTML Canvas 2D with path batching |
| Worker Comm | postMessage with transferable ArrayBuffers |
| Deployment | Vercel |

## Development

### Prerequisites

- Node.js 18+
- pnpm
- Rust toolchain with `wasm32-unknown-unknown` target
- wasm-pack

### Setup

```bash
# Install dependencies
pnpm install

# Build WASM (only needed when Rust code changes)
pnpm build:wasm

# Start dev server
pnpm dev
```

### Commands

| Command | Description |
|---------|------------|
| `pnpm dev` | Start Next.js dev server |
| `pnpm build` | Production build |
| `pnpm build:wasm` | Rebuild WASM binary from Rust |
| `pnpm build:all` | Build WASM + Next.js |

### Running Rust Tests

```bash
cd crates/simulation
cargo test
```

Tests cover determinism, population stability, energy conservation, and NaN resilience.

## Controls

| Input | Action |
|-------|--------|
| Space | Play / Pause |
| . | Single step |
| 1-5 | Speed presets |
| Scroll | Zoom |
| Drag | Pan |

## Architecture

See [ARCHITECTURE.md](./ARCHITECTURE.md) for the full design document covering energy systems, physics, neural networks, genetics, speciation, rendering pipeline, security model, and failsafe architecture.

## License

MIT
