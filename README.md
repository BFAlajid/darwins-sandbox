# Darwin's Sandbox

An agent-based simulation of soil-transmitted helminth (STH) infection and deworming, running entirely in the browser. It is calibrated to the findings of a thesis on school-aged children in Cebu City, Philippines (Ascaris, Trichuris and hookworm; urban vs. rural barangays).

The simulation engine is Rust compiled to WebAssembly, running in a Web Worker. The interface is Next.js with Canvas 2D / WebGL2 rendering.

## What it models

- Individual agents (children, parents, teachers, health workers) following daily routines in urban and rural barangays
- Transmission of Ascaris, Trichuris and hookworm through a contaminated environment grid
- Knowledge, attitudes and practices (KAP) per agent, alongside structural factors such as latrine and water access
- Interventions: mass drug administration (MDA), WASH, education and health-worker visits, with a cost tracker
- Urban and rural side-by-side comparison
- Prevalence history charted against the thesis reference values (Ascaris 11.7%, Trichuris 4.8%, hookworm 0%)
- CSV export (prevalence in percent, columns suffixed `_pct`) and JSON stats snapshot

Reference thesis data lives in `src/lib/thesis-data.ts`. The design is described in [STH-ARCHITECTURE.md](./STH-ARCHITECTURE.md).

## Tech stack

| Layer | Technology |
|-------|-----------|
| Simulation | Rust to WASM (`wasm-bindgen`, `serde`, `slotmap`, `rand`) in `crates/simulation` |
| Frontend | Next.js 14, React, TypeScript, Tailwind CSS |
| State | Zustand |
| Worker bridge | Comlink / `postMessage` |
| Deployment | Vercel |

## Development

Prerequisites: Node.js, pnpm, a Rust toolchain with the `wasm32-unknown-unknown` target, and `wasm-pack`.

```bash
pnpm install
pnpm build:wasm   # only needed when the Rust code changes
pnpm dev
```

| Command | Description |
|---------|-------------|
| `pnpm dev` | Start the Next.js dev server |
| `pnpm build` | Production build |
| `pnpm start` | Serve the production build |
| `pnpm build:wasm` | Build the crate with `wasm-pack` and copy it to `public/wasm` (and the JS glue to `src/workers/wasm`) |
| `pnpm build:all` | `build:wasm` then `build` |

The worker loads the committed binary from `public/wasm/`, so rebuild and commit it after any change to the Rust engine.

### Tests

```bash
cd crates/simulation && cargo test        # engine unit and calibration tests
node --test src/lib/prevalence.test.ts    # percent conversion of engine output
npx tsc --noEmit                          # type check
```

## License

MIT
