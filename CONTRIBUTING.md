# Contributing to Slay ⚔️

First off, thank you for considering contributing to Slay! We aim to build the best open-source sandbox for system design enthusiasts.

## Prerequisites

- **Rust**: Ensure you have the latest stable version of Rust installed.
  ```bash
  rustup update stable
  ```
- **Cargo**: Comes with Rust.
- **WASM Target** (for web development):
  ```bash
  rustup target add wasm32-unknown-unknown
  ```
- **Trunk** (for local web server):
  ```bash
  cargo install --locked trunk
  ```
- **Docker** (optional): For containerized builds.

## Code Style

We strictly follow standard Rust formatting.
**Before submitting a PR, always run:**
```bash
cargo fmt
```

## The "Mirror" Architecture

Slay uses a strict separation between **Simulation Logic** (`core`) and **Visualization** (`ui`). This allows the simulation to run headless (e.g., for CI/CD or reinforcement learning) without any UI dependencies.

To add a new component (e.g., `Database`), you need to implement it in both layers:

### 1. Implement Logic (The "Brain")
**Location**: `core/src/components/` (e.g., `database.rs`)

1.  **Define Stats Struct**: Create a `DatabaseStats` struct with the fields you want to display on the canvas (RPS, queue length, etc.).
    - Must derive `Debug`, `Clone`, `Serialize`, `Deserialize`.
2.  **Define Config**: Create a `DatabaseConfig` struct for runtime-configurable parameters.
    - Must derive `Serialize`, `Deserialize`, `Clone`, `Debug` and implement `Default`.
3.  **Define Struct**: Create the main `Database` struct, holding `config: Arc<RwLock<DatabaseConfig>>`, metrics counters, and `display_snapshot: VisualState`.
4.  **Implement `Component` Trait**:
    - `on_event`: Core logic. Return `Vec<ScheduleCmd>` to schedule future events.
    - `encode_config` / `apply_config`: Bridge to the UI inspector.
    - `sync_display_stats`: Update sliding windows and set `self.display_snapshot = VisualState::Database(DatabaseStats { ... })`.
    - `get_visual_snapshot`: Return `self.display_snapshot.clone()`.
    - Other required methods: `name`, `kind`, `active_requests`, `display_throughput`, `error_count`, `set_healthy`, `is_healthy`, `add_target`, `remove_target`, `get_targets`, `clear_targets`, `reset_internal_stats`, `set_seed`.

### 2. Register Logic
**Location**: `core/src/components/mod.rs`

- Add `pub mod database;`
- Register in `register_components!` with **both** the component type and the stats type:
  ```rust
  register_components!(
      Client    => client::Client,    client::ClientStats,
      Server    => server::Server,    server::ServerStats,
      LoadBalancer => load_balancer::LoadBalancer, load_balancer::LBStats,
      Database  => database::Database, database::DatabaseStats,  // ← add this
  );
  ```
  The macro automatically:
  - Generates a `create_component(kind: &str, ...)` factory function.
  - Adds a `Database(DatabaseStats)` variant to the `VisualState` enum.

### 3. Implement View (The "Face")
**Location**: `ui/src/components/` (e.g., `database.rs`)

1.  **Define View Struct**: Create an empty struct (e.g., `DatabaseView`).
    - Derive `Default`.
2.  **Implement `ComponentView` Trait** (using `egui`):
    - `render_canvas`: Draw the node on the graph. Use the `snapshot` data (from `get_visual_snapshot`) to show real-time stats like RPS or queue size.
    - `render_inspector`: Draw the configuration UI (sliders, checkboxes) in the side panel. Return `true` if any config changed.

### 4. Register View
**Location**: `ui/src/components/mod.rs`

- Add `pub mod database;`
- Register it in the `register_views!` macro:
  ```rust
  "Database" => database::DatabaseView,
  ```

---

## Development Workflow

1.  **Build**:
    ```bash
    cargo build
    ```
2.  **Run UI**:
    ```bash
    cargo run -p slay-ui
    ```
3.  **Run Core Tests**:
    ```bash
    cargo test -p slay-core
    ```
4.  **Run UI Tests**:
    ```bash
    cargo test -p slay-ui
    ```

### Web (WASM) Development

To run the application in a browser with hot-reloading:
```bash
cd ui
trunk serve
```

### Docker (Easiest Way)

You can run the full simulator stack using the pre-built image from GitHub Container Registry:

```bash
docker compose up -d
```

To build locally (e.g. validting changes):
```bash
docker compose up --build
```

### Core Design Principles

1.  **Physics First**: Every delay (network, processing) must be accounted for in virtual time (`event.time`).
2.  **Headless-Ready**: The `core` crate **must never** depend on GUI libraries (`egui`, `winit`).
3.  **Visual Stability**: The UI renders based on intermittent "snapshots" from the core. Ensure your `get_visual_snapshot` returns data that doesn't jitter wildly.
4.  **Shared State**: Use `Arc<RwLock<Config>>` for parameters that can be changed at runtime by the UI.
5.  **Determinism**: All randomness must come from the `Component`'s seeded RNG (via `set_seed`). Never use `rand::thread_rng()` or `SystemTime` for logic.

## Questions?
Open an issue or start a discussion on GitHub!