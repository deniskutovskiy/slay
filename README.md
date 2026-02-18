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
- [🚀 Implementation Roadmap](#-implementation-roadmap)
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
*   **UI:** [egui](https://github.com/emilk/egui) for a high-performance, immediate-mode interface.
*   **Architecture:** Modular workspace-based design with strict separation between simulation logic (`core`) and visualization (`ui`).
*   **Visual Stability:** Global "Visual Snapshot" system to ensure smooth, human-readable metrics without flickering.
*   **Determinism:** Seeded RNG for reproducible simulation runs (debugging made easy).

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
| **Client** | ✅ Active | RPS (λ) load source, Request Timeouts, Configurable load patterns. |
| **App Server** | ✅ Active | Thread pools, Backlog Limit, Service Time Jitter. |
| **Load Balancer** | ✅ Active | Round-robin, Random, Least-connections, Stateful tracking. |
| **Database** | ⏳ TODO | Replication (Sync/Async), Sharding, Lock contention. |

---

## 🚀 Implementation Roadmap

### Phase 1: Foundation (Completed)
- [x] Core Event Loop with Min-Heap for time management.
- [x] Infinite Canvas with Panning and Zooming.
- [x] Modular "Mirror" architecture for components.
- [x] Automated `TestHarness` for system realism validation.

### Phase 2: Topology & Logic (In Progress)
- [x] Connection editor with high-performance pulse animations.
- [x] Load Balancer implementation with stateful tracking.
- [x] Component health manipulation (Manual node failure injection).
- [ ] Metric export (Prometheus/Grafana compatible formats).

### Phase 3: Advanced Network (Next)
- [x] **Edges as Entities**: Move network properties (Latency, Packet Loss) into individual connections.
- [ ] Region/AZ simulation (Network penalties for cross-zone traffic).
- [x] Real-time line charts for performance trends in the dashboard.

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
