# Contributing to Slay ⚔️

First off, thank you for considering contributing to Slay! We aim to build the best open-source sandbox for system design enthusiasts.

## The "Mirror" Architecture

Slay uses a strict separation between **Simulation Logic** and **Visualization**. 

To add a new component (e.g., `Database`), follow this logical flow:

### 1. Implement Logic
**Create file**: `core/src/components/database.rs`
- Define `struct DatabaseConfig` (must implement `Serialize, Deserialize, Default`).
- Define `struct Database` and implement the `Component` trait.
- Set defaults via `impl Default for Database`.

### 2. Register Logic
**Edit file**: `core/src/components/mod.rs`
- Add `pub mod database;`
- Add a line to `register_components!`:
  ```rust
  "Database" => database::Database,
  ```

### 3. Implement View
**Create file**: `ui/src/components/database.rs`
- Define `struct DatabaseView` and implement the `ComponentView` trait using `egui`.
- Define how the node looks on the canvas and what sliders appear in the inspector.

### 4. Register View
**Edit file**: `ui/src/components/mod.rs`
- Add `pub mod database;`
- Add a line to `register_views!`:
  ```rust
  "Database" => database::DatabaseView,
  ```

---

## Development Workflow

- **Build**: `cargo build`
- **Run UI**: `cargo run -p slay-ui`
- **Test Core**: `cargo test -p slay-core`
- **Test UI**: `cargo test -p slay-ui`

### Core Design Principles
1.  **Physics First**: Every delay (network, processing) must be accounted for in virtual time.
2.  **Headless-Ready**: The `core` crate must never depend on GUI libraries (`egui`).
3.  **Visual Stability**: Use JSON snapshots for canvas rendering to prevent flickering.
4.  **Shared State**: Use `Arc<RwLock<Config>>` for parameters that the UI can change.

## Questions?
Open an issue or start a discussion on GitHub!