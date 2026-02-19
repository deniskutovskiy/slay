# ⚔️ Slay: Interactive System Design Simulator

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/language-Rust-orange.svg)](https://www.rust-lang.org/)

**Slay** is a high-fidelity, Open Source sandbox simulator designed for engineers to architect, simulate, and stress-test distributed systems using **Discrete Event Simulation (DES)**.

> *“Don’t just design your SLA — Slay it.”*

---

## 📖 Table of Contents
- [🎯 Concept & Objectives](#-concept--objectives)
- [🛠 Tech Stack](#-tech-stack)
- [🧮 Mathematical Simulation Model](#-mathematical-simulation-model)
- [📦 Component Library](#-component-library)
- [� Roadmap](ROADMAP.md)
- [🛠 Getting Started](#-getting-started)
- [🤝 Contributing](#-contributing)

---

## 🎯 Concept & Objectives

The project provides an interactive environment where architectural decisions are validated through real-time load and failure simulations. It models system physics—queues, thread pools, and network hops—to provide a realistic sandbox for System Design and SRE practices.

### Core KPIs:
*   **Availability (SLA):** Success/Failure ratio calculated via sliding time windows.
*   **Performance:** High-resolution latency tracking ($P50$, $P95$, $P99$).
*   **Saturation:** Monitoring of thread pool exhaustion and backlog overflows.

---

## 🛠 Tech Stack

*   **Engine:** Rust-based Discrete Event Simulation (DES) using a `BinaryHeap` priority queue.
*   **UI:** [egui](https://github.com/emilk/egui) for a high-performance, immediate-mode interface, compiled to native and WASM.
*   **Architecture:** Modular workspace-based design with strict separation between simulation logic (`core`) and visualization (`ui`).
*   **Visual Stability:** Global Visual Snapshot system — components generate a `VisualState` enum (auto-generated via `register_components!` macro) for smooth, flicker-free metrics.
*   **Determinism:** Seeded RNG for reproducible simulation runs.

---

## 🧮 Mathematical Simulation Model

### 3.1. Discrete Event Simulation (DES)
Unlike frame-based game engines, Slay jumps between discrete events (Arrival, ProcessComplete, Response). This allows simulating millions of RPS without CPU overhead by only processing meaningful state changes.

### 3.2. Response Routing (Call Stack)
Requests carry a `path: Vec<NodeId>` trace. Nodes push themselves onto the stack during the "forward" hop and pop themselves to route the response back. This ensures realistic RTT (Round Trip Time) calculation across any topology.

### 3.3. Headless Logic
The core simulation engine has **zero** dependencies on the UI layer. It communicates via JSON snapshots, allowing the engine to run in headless environments (CLI, CI/CD) or with different frontend implementations.

---

## 📦 Component Library

| Component | Status | Features |
| :--- | :--- | :--- |
| **Client** | ✅ Active | RPS (λ) load source, request timeouts, jitter. |
| **App Server** | ✅ Active | Thread pools, backlog limit, saturation penalty, service time jitter. |
| **Load Balancer** | ✅ Active | Round-robin, Random, Least-connections; retry with token budget, per-request failure tracking. |
| **Database** | ⏳ Planned | Replication (Sync/Async), sharding, lock contention. |
| **Cache** | ⏳ Planned | Hit/miss simulation, TTL eviction, cache stampede. |

---

## � Roadmap

See [ROADMAP.md](ROADMAP.md) for the full development plan.

---

## 🛠 Getting Started

### Prerequisites
*   [Rust](https://www.rust-lang.org/tools/install) (latest stable) - *Only if building from source*
*   [Docker](https://www.docker.com/) - *Recommended for quick start*

### ⚡️ Try Online (WASM)
👉 **[slay.ktvsk.ru](https://slay.ktvsk.ru/)** — Run directly in your browser without installation.

### ⚡️ Quick Start (Docker)
The quickest way to run locally is using the pre-built Docker image:
```bash
docker compose up -d
```
Open [http://localhost:8080](http://localhost:8080) to start slaying.

### 🛠 Building from Source
```bash
# Run the simulator with the default topology
cargo run -p slay-ui
```

### Running Core Tests
```bash
cargo test -p slay-core
```

---

## 🤝 Contributing

Contributions are welcome! Slay uses a mirrored architecture where logic and view are strictly separated. 

Please see [CONTRIBUTING.md](CONTRIBUTING.md) for a guide on how to add new components.

---

## 📜 License

MIT License - see the [LICENSE](LICENSE) file for details.
