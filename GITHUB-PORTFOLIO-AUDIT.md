# GitHub Portfolio Audit — BFAlajid (Basil Francis Alajid)

**Generated:** 2026-03-17
**GitHub:** https://github.com/BFAlajid
**Public Repos:** 8

---

## Portfolio Summary

| Repo | Primary Language | Lines of Code | Commits | Stars | Live URL | Status |
|------|-----------------|---------------|---------|-------|----------|--------|
| professor-basils-lab | TypeScript + Rust | ~2.8M bytes | 70 | 3 | [professor-basils-lab.vercel.app](https://professor-basils-lab.vercel.app) | Active |
| portfolio | TypeScript | ~403K bytes | 35 | 3 | [basilfrancis-alajid.vercel.app](https://basilfrancis-alajid.vercel.app) | Active |
| manila-watch-atelier-MVP | TypeScript | ~820K bytes | 13 | 3 | [manila-watch-atelier-mvp.vercel.app](https://manila-watch-atelier-mvp.vercel.app) | Active |
| darwins-sandbox | Rust + TypeScript | ~138K bytes | 9 | 1 | [darwins-sandbox.vercel.app](https://darwins-sandbox.vercel.app) | Active |
| playwright-archaeologist | TypeScript | ~964K bytes | 1 | 0 | npm package | New |
| auditfix | TypeScript | ~428K bytes | 22 | 1 | npm package | Active |
| dev-savestate | TypeScript | ~130K bytes | 5 | 0 | npm package | Active |
| BFAlajid | — (profile README) | — | 8 | 1 | GitHub profile | Maintained |

---

## Detailed Repository Audits

---

### 1. Professor Basil's Lab
**https://github.com/BFAlajid/professor-basils-lab**

**One-liner:** Full-stack Pokemon platform — team builder, battle simulator, GBA/NDS/3DS emulators, and competitive analysis tools — all client-side with Rust/WASM acceleration.

**Tech Stack:**
- TypeScript (1.94M bytes), JavaScript (544K), Rust (164K), Makefile (155K)
- Next.js, React, Tailwind CSS, Tanstack Query, Framer Motion
- Rust/WASM (10 crates), PeerJS (P2P multiplayer), IndexedDB
- Vercel KV (leaderboards)

**Key Features:**
- Team builder with full EVs/IVs/natures/abilities customization for all 1,025 Pokemon
- Turn-based battle engine: full damage formula, 100+ abilities, 50+ held items, weather, status, Mega Evolution, Terastallization
- 5 AI difficulty tiers, online PvP via PeerJS, 8 Kanto gym leaders, Elite Four, Battle Tower/Factory, tournaments
- Full replay system with step-through viewer
- ELO ranked ladder with global leaderboards
- GBA emulator (ARM7TDMI + PPU), NDS emulator (in progress), 3DS emulator (HLE)
- Wild encounters, breeding system, Safari Zone, Nuzlocke mode
- PC storage (30 boxes), Pokedex, evolution tree viewer
- PWA-capable, offline-first

**Branch Structure:**
| Branch | Status | Ahead/Behind Master |
|--------|--------|-------------------|
| master | Default, production | — |
| main | Mirror | — |
| develop | Integration | +2 / -2 |
| feature/citrine | 3DS emulator (HLE) | +8 / -25 |
| feature/multipokemon | Overworld multiplayer | +2 / -14 |
| feature/implementations | IDE lint fixes | +3 / -25 |
| feature/expressive-jingling-lightning | Server pkg setup | +2 / -15 |
| feature/rust-refactor | Lint cleanup (merged) | +0 / -53 |

**Portfolio Talking Points:**
- Demonstrates systems-level programming (ARM7TDMI emulation in Rust/WASM)
- Complex state management (battle engine with multi-turn prediction)
- Real-time P2P networking (PeerJS for online battles)
- Performance optimization (WASM acceleration for CPU-intensive emulation)
- Full-stack: client rendering, server functions, KV storage, P2P

**Scale:** Largest project — nearly 2M bytes of TypeScript alone, 70 commits, 10 Rust crates.

---

### 2. Portfolio Website
**https://github.com/BFAlajid/portfolio**

**One-liner:** Personal developer portfolio with 3D interactive elements, smooth animations, and MDX blog — showcasing engineering depth and design sensibility.

**Tech Stack:**
- TypeScript (334K), MDX (60K), SCSS (4.3K), CSS (3.6K)
- Next.js 14 (App Router, SSG), Tailwind CSS, Shadcn UI
- GSAP + Framer Motion (animations), Spline (3D interactive keyboard)
- Resend (contact form), Lenis (smooth scrolling)
- next-themes (dark/light mode)

**Key Features:**
- Hero section with Spline 3D interactive keyboard
- GSAP scroll-driven animations throughout
- Engineering philosophy section (`/approach`)
- Project case study deep dives with prev/next navigation
- MDX blog with code syntax highlighting
- Contact form with email delivery (Resend)
- Dark/light mode toggle
- Responsive design

**Branch Structure:**
| Branch | Status | Ahead/Behind Master |
|--------|--------|-------------------|
| master | Default, production | — |
| main | Mirror | — |
| dev | Feature branch | +1 / -7 (testimonials, /uses, RSS feed, 15+ UX improvements) |

**Portfolio Talking Points:**
- Demonstrates design sense and attention to UX detail
- Complex animation choreography (GSAP + Framer Motion)
- 3D web integration (Spline)
- Content pipeline (MDX for blog)
- SEO-aware SSG architecture

---

### 3. Manila Watch Atelier MVP
**https://github.com/BFAlajid/manila-watch-atelier-MVP**

**One-liner:** Luxury watch e-commerce platform for a Manila-based grey market dealer — inventory showcase, inquiry pipeline, market insights, and admin dashboard.

**Tech Stack:**
- TypeScript (739K), CSS (59K), JavaScript (20K), HTML (2.5K)
- React 18, Vite, Tailwind CSS, Framer Motion, Radix/Shadcn
- PostgreSQL + Prisma ORM (later migrated to JSON-file backend for Vercel)
- Resend (email), Recharts (market insights)
- SEO: react-helmet-async, JSON-LD, dynamic sitemap

**Key Features:**
- Inventory showcase with filtering (brand, price, condition, search)
- Watch detail pages with image gallery, video, specifications
- Psychology-driven conversion elements (FOMO, scarcity, urgency, social proof)
- Inquiry system with database storage, email notifications, rate limiting
- WhatsApp integration (pre-filled messages per watch)
- Admin dashboard: CRUD, status pipeline (New/Contacted/Closed)
- Market Insights page with interactive charts (brand distributions, grey market premiums)
- SEO-optimized brand landing pages
- Multi-currency display (PHP, USD, EUR, SGD, HKD)
- Client-side favorites, comparison, recently viewed

**Branch Structure:**
| Branch | Status | Ahead/Behind Master |
|--------|--------|-------------------|
| master | Default, production | — |
| main | Mirror | — |
| development | Security hardening + cleanup | +1 / -1 |

**Portfolio Talking Points:**
- Real client project (not a toy — built for a Manila-based business)
- Full business logic: inventory management, lead generation, admin pipeline
- E-commerce UX: conversion psychology, multi-currency, WhatsApp integration
- Data visualization (Recharts market insights)
- Backend migration story (Prisma → JSON for Vercel deployment constraints)

---

### 4. Darwin's Sandbox
**https://github.com/BFAlajid/darwins-sandbox**

**One-liner:** Real-time evolution simulator (now refactored to STH infection/deworming simulation) with Rust/WASM engine, agent-based modeling, and Canvas 2D rendering.

**Tech Stack:**
- Rust (76K bytes), TypeScript (29K), JavaScript (32K)
- Rust → WASM (292KB binary), wasm-bindgen, slotmap, rand, serde
- Next.js 14, Zustand, Canvas 2D
- Web Worker (off-main-thread simulation)

**Key Features (Evolution Sim):**
- Closed energy budget with carrying capacity emergence
- Neural network brains (16→8→4 feedforward)
- Genetic mutation (Gaussian + Cauchy, heritable mutation rates)
- Speciation via genome distance clustering
- Canvas rendering with LOD system, species color batching
- Interventions: meteor, drought, flood, wall drawing, predator mode

**Key Features (STH Simulation — local, not yet pushed):**
- Agent-based STH infection model (Ascaris, Trichuris, Hookworm)
- Stochastic SIS transmission with WHO EPG thresholds
- KAP behavioral model with structural dominance (thesis finding)
- 6 intervention types (MDA, WASH, Education, BHW, Budget, Policy)
- Environmental contamination grid with decay/rain dynamics
- Comparison mode (urban vs rural side-by-side)
- Calibrated against real thesis data from Cebu City

**Branch Structure:**
| Branch | Status |
|--------|--------|
| main | Default, production |

**Portfolio Talking Points:**
- Systems-level WASM engineering (deterministic simulation, polynomial approximations for cross-browser consistency)
- Agent-based modeling (epidemiological domain)
- Research-to-software pipeline (medical thesis → interactive simulation)
- Performance: 500 agents at 60fps in <1ms per tick

---

### 5. Playwright Archaeologist
**https://github.com/BFAlajid/playwright-archaeologist**

**One-liner:** CLI tool that generates a complete behavioral specification of any running web app — sitemap, form catalog, API map (OpenAPI 3.0), screenshots, flow graphs, and regression diffs — no source code required.

**Tech Stack:**
- TypeScript (964K bytes)
- Playwright (headless Chromium), CLI
- Published to npm as `playwright-archaeologist`

**Key Features:**
- Zero source code access — works on any running web app
- Generates: sitemap, form catalog, API map (OpenAPI 3.0 schema)
- Screenshots of every page
- Navigation flow graph
- Regression baseline with diff capability
- CLI: `pa dig https://example.com`

**Branch Structure:**
| Branch | Status |
|--------|--------|
| master | Default |

**Portfolio Talking Points:**
- Developer tooling / DX focus
- Published npm package
- Reverse engineering approach (behavioral specification from runtime observation)
- Practical testing infrastructure tool

---

### 6. AuditFix
**https://github.com/BFAlajid/auditfix**

**One-liner:** Smarter npm dependency security CLI that replaces `npm audit` with production reachability analysis, risk scoring, supply chain intelligence, and safe auto-fixes.

**Tech Stack:**
- TypeScript (428K bytes)
- OSV.dev API (real-time advisory matching)
- Published to npm as `auditfix`
- GitHub Actions CI

**Key Features:**
- Production reachability analysis (distinguishes dev-only vs production dependencies)
- Risk scoring with EPSS scores and CISA KEV matching
- Supply chain intelligence
- Safe auto-fix (`auditfix --fix`)
- PR comment bot (`--pr-comment`) for GitHub Actions
- Teams/Discord webhook notifications
- SARIF diff mode for CI/CD integration
- Severity classification with actionable remediation

**Branch Structure:**
| Branch | Status |
|--------|--------|
| main | Default |

**Portfolio Talking Points:**
- Security engineering depth
- CI/CD integration (PR bots, webhooks, SARIF)
- Real-world developer tooling published to npm
- Supply chain security awareness

---

### 7. Dev Savestate
**https://github.com/BFAlajid/dev-savestate**

**One-liner:** CLI tool that saves and restores your full development working context (git state, open files, cursor positions, running processes) — like emulator save states for dev environments.

**Tech Stack:**
- TypeScript (130K bytes)
- Node.js 20+, Git integration
- Published to npm as `dev-savestate`

**Key Features:**
- Save/restore full working context: git changes, open files, cursor positions, running processes, context notes
- `dvs save`, `dvs load`, `dvs list`, `dvs back` (undo last load)
- Sub-second context switching
- Addresses the "20-30 minutes per context switch" problem

**Branch Structure:**
| Branch | Status |
|--------|--------|
| main | Default |

**Portfolio Talking Points:**
- Developer productivity tooling
- Deep git integration
- Solves a real pain point (context switching cost)
- Clean CLI design

---

### 8. BFAlajid (Profile README)
**https://github.com/BFAlajid/BFAlajid**

GitHub profile README with animated header, typing SVG, trophy display, and dark/light mode stats.

---

## Technology Skill Matrix (Derived from Repos)

| Category | Technologies | Evidence |
|----------|-------------|----------|
| **Languages** | TypeScript, Rust, JavaScript, HTML, CSS, SCSS, MDX | All repos |
| **Frontend Frameworks** | Next.js 14, React 18, Vite | portfolio, professor-basils-lab, manila-watch-atelier |
| **Styling** | Tailwind CSS, Shadcn/Radix, Framer Motion, GSAP | portfolio, professor-basils-lab, manila-watch-atelier |
| **Systems Programming** | Rust, WASM (wasm-bindgen, wasm-pack), ARM7TDMI emulation | darwins-sandbox, professor-basils-lab |
| **State Management** | Zustand, Tanstack Query | darwins-sandbox, professor-basils-lab |
| **Backend** | Vercel Serverless, PostgreSQL, Prisma, Vercel KV, Resend | manila-watch-atelier, portfolio, professor-basils-lab |
| **DevOps/CI** | GitHub Actions, Vercel, npm publishing | auditfix, all deployed projects |
| **Testing/QA** | Playwright, behavioral specification | playwright-archaeologist |
| **Security** | npm audit replacement, EPSS scoring, supply chain analysis, SARIF | auditfix |
| **Networking** | PeerJS (P2P WebRTC), Web Workers | professor-basils-lab, darwins-sandbox |
| **Data Viz** | Canvas 2D, Recharts, radar charts, heatmaps | darwins-sandbox, manila-watch-atelier, professor-basils-lab |
| **3D/Graphics** | Spline, WebGL2, Canvas rendering pipelines | portfolio, darwins-sandbox |
| **Developer Tooling** | CLI tools (3 npm packages), VSCode integration | auditfix, dev-savestate, playwright-archaeologist |
| **Domain Expertise** | Epidemiological modeling, game emulation, e-commerce, security | darwins-sandbox, professor-basils-lab, manila-watch-atelier, auditfix |

---

## Aggregate Statistics

- **Total public repos:** 8
- **Total commits:** 163
- **Total stars:** 12
- **Languages used:** TypeScript (dominant), Rust, JavaScript, SCSS, MDX, CSS, HTML, Shell, Makefile
- **npm packages published:** 3 (auditfix, dev-savestate, playwright-archaeologist)
- **Live deployments:** 4 (Vercel)
- **Active feature branches:** 6 (across professor-basils-lab and portfolio)
- **Largest project:** professor-basils-lab (~2.8M bytes, 70 commits, 10 Rust crates)

---

## Portfolio Narrative Themes

1. **Full-stack depth** — Not just CRUD. Systems programming (WASM, ARM emulation), complex state machines (battle engine), real-time networking (P2P), and agent-based simulation.

2. **Real clients, real users** — Manila Watch Atelier is a production business tool, not a tutorial clone. Portfolio is a polished personal brand.

3. **Developer tooling mindset** — Three published npm packages (auditfix, dev-savestate, playwright-archaeologist) solving real developer problems.

4. **Security awareness** — AuditFix demonstrates deep understanding of supply chain security, EPSS scoring, and CI/CD integration.

5. **Research-to-software pipeline** — Darwin's Sandbox/STH simulation bridges academic research (medical thesis) and interactive software. Demonstrates ability to translate domain knowledge into working systems.

6. **Performance engineering** — WASM optimization, spatial hashing, double-buffered ArrayBuffers, LOD rendering — consistent attention to runtime performance across projects.

7. **Breadth of domains** — Gaming (Pokemon), e-commerce (watches), public health (STH), developer tools, security — demonstrates adaptability.
