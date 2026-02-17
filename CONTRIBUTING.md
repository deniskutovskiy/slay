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

1.  **Define Config**: Create a struct (e.g., `DatabaseConfig`) that holds the component's parameters (latency, capacity, etc.).
    - Must derive `Serialize`, `Deserialize`, `Clone`, `Debug`.
    - Implement `Default`.
2.  **Define Struct**: Create the main struct (e.g., `Database`).
3.  **Implement `Component` Trait**:
    - `on_event`: The core logic. Handle `Arrival`, `ProcessComplete`, etc. Return `ScheduleCmd`s to schedule future events.
    - `name`, `kind`: Identifiers.
    - `palette_color_rgb`: The color used in the UI for this node type.
    - `encode_config` / `apply_config`: Bridge between the UI inspector and the internal state.
    - `get_visual_snapshot`: Return a JSON object with metrics (RPS, queue length) for the UI to render.
    - `sync_display_stats`: Update internal sliding windows for smooth metrics.

### 2. Register Logic
**Location**: `core/src/components/mod.rs`

- Add `pub mod database;`
- Register it in the `register_components!` macro:
  ```rust
  "Database" => database::Database,
  ```

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

## Questions?
Open an issue or start a discussion on GitHub!