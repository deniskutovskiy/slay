# 🗺 Slay Roadmap

This document tracks the high-level development direction of Slay. Items marked ✅ are shipped; 🔄 are in progress; ⏳ are planned.

---

## ✅ Phase 1 — Foundation
Core infrastructure to make the simulation engine and UI work together.

- [x] Discrete Event Simulation engine with `BinaryHeap` time management
- [x] Infinite canvas with pan and zoom
- [x] Mirror architecture: headless `core` + `egui` UI layer
- [x] `TestHarness` for integration-level simulation tests
- [x] Seeded RNG for reproducible, deterministic runs
- [x] Native desktop binary + WASM build (via Trunk)

---

## ✅ Phase 2 — Topology & Logic
The building blocks for realistic distributed system simulations.

- [x] Connection editor with animated traffic pulses
- [x] Load Balancer: Round-Robin, Random, Least-Connections
- [x] Node health controls: manual failure injection
- [x] Edges as Entities: per-link latency, jitter, and packet loss
- [x] Real-time latency line charts in the dashboard
- [x] Retry logic in LB: token budget, backoff, per-request failure tracking
- [x] `register_components!` macro: single registration generates `VisualState` enum and component factory
- [x] Canvas badges: active retries (↻), failure count (x N) on LB node

---

## 🔄 Phase 3 — Advanced Components
Filling out the component library with stateful backend primitives.

- [ ] **Database** node
  - Read/write split (primary + replicas)
  - Replication lag simulation (sync vs async)
  - Lock contention and queue buildup
- [ ] **Cache** node
  - Configurable hit ratio and TTL
  - Cache stampede / thundering herd simulation
  - Bypass mode for cache-miss propagation to DB
- [ ] **Queue / Message Broker** node
  - Producer → Consumer model
  - Configurable consumer concurrency and dead-letter policy

---

## ⏳ Phase 4 — Observability & Export
Making simulation data actionable outside the UI.

- [ ] Prometheus-compatible metric scrape endpoint (headless mode)
- [ ] JSON snapshot export for post-run analysis
- [ ] Save/Load topology as JSON scene file
- [ ] Scenario replay: seed + event log deterministic replay

---

## ⏳ Phase 5 — Multi-Region & Advanced Topology
- [ ] Region / Availability Zone grouping with cross-zone latency penalties
- [ ] Circuit Breaker node
- [ ] Rate Limiter node (token bucket, sliding window)
- [ ] Auto-scaling simulation: thread pool grows/shrinks under load