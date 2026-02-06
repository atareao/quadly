# Quadly Project - AI Coding Assistant Instructions

## Overview
**Quadly** is a web-based management interface for Podman Quadlets - systemd-compatible container configurations. The project features a Rust/Axum backend that interfaces with systemd via D-Bus and a React/TypeScript frontend.

## Architecture

### Backend Structure (`backend/`)
- **Entry Point**: [backend/src/main.rs](../backend/src/main.rs) - Axum server with rate limiting and CORS
- **API Layer**: [backend/src/api/](../backend/src/api/) - REST endpoints for quadlet management 
- **System Integration**: [backend/src/system/](../backend/src/system/) - systemd D-Bus communication, file storage, and log retrieval
- **Core Logic**: [backend/src/core/](../backend/src/core/) - Pest parser for quadlet file validation
- **Models**: [backend/src/models/](../backend/src/models/) - Shared data structures with TypeScript bindings

### Frontend Structure (`frontend/`)
- Standard React + Vite + TypeScript setup
- **Type Safety**: Backend generates TypeScript bindings via `ts-rs` crate to `frontend/src/bindings/`
- **API Integration**: Communicates with backend at `http://127.0.0.1:3000/api/v1`

## Key Patterns & Conventions

### D-Bus Integration Pattern
The project uses **rootless systemd** (user session) exclusively:
```rust
let conn = Connection::session().await?;  // Always session bus, never system
```
- All quadlet operations target `~/.config/containers/systemd/`
- Service names follow pattern: `{quadlet_name}.service`
- Actions: `start`, `stop`, `restart`, `daemon-reload`

### API Response Pattern
Consistent error handling in handlers:
```rust
match system::operation() {
    Ok(result) => (StatusCode::OK, Json(result)).into_response(),
    Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
}
```

### File Storage Convention
- **Quadlet files**: `~/.config/containers/systemd/*.container`
- **File operations**: Always use [backend/src/system/storage.rs](../backend/src/system/storage.rs) functions
- **systemd integration**: Call `daemon-reload` after file modifications

### Type Generation Workflow
Models in [backend/src/models/](../backend/src/models/) use `ts-rs` derive macro:
```rust
#[derive(Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/QuadletStatus.ts")]
pub enum QuadletStatus { ... }
```
Run `cargo build` to regenerate TypeScript bindings.

## Development Workflows

### Backend Development
```bash
cd backend
cargo run          # Start server on localhost:3000
cargo check        # Fast compilation check
cargo clippy       # Linting
```

### Frontend Development
```bash
cd frontend
pnpm dev           # Start dev server with HMR
pnpm build         # Production build
pnpm lint          # ESLint
```

### Testing Quadlet Integration
Ensure you have podman and systemd --user enabled:
```bash
systemctl --user status    # Verify user systemd is running
podman --version           # Verify podman is installed
```

## Critical Implementation Details

### systemd Proxy Definitions
The project uses custom zbus proxy traits in [backend/src/system/systemd.rs](../backend/src/system/systemd.rs):
- `SystemdManager` - Main systemd interface
- `SystemdUnit` - Individual service control
- Always use `"replace"` mode for unit operations

### Spanish Comments
The codebase contains Spanish comments and variable names - maintain this convention for consistency with existing team practices.

### Error Context
Use `anyhow::Context` for meaningful error messages:
```rust
.context("Failed to connect to systemd session bus")?
```

### Async Patterns
- Use `join_all()` for concurrent status queries across multiple quadlets
- Background monitoring via `tokio::sync::broadcast` channels for real-time updates
- SSE implementation for frontend live updates

## File Organization Rules

- **New API endpoints**: Add to [backend/src/api/handlers.rs](../backend/src/api/handlers.rs), export in [mod.rs](../backend/src/api/mod.rs)
- **systemd operations**: Extend [backend/src/system/systemd.rs](../backend/src/system/systemd.rs)
- **Storage operations**: Extend [backend/src/system/storage.rs](../backend/src/system/storage.rs)
- **Parser extensions**: Modify [backend/src/core/quadlet.pest](../backend/src/core/quadlet.pest) grammar

## Dependencies Notes
- **zbus**: D-Bus communication - always use session connection
- **pest**: Grammar parsing - see [quadlet.pest](../backend/src/core/quadlet.pest) for syntax
- **ts-rs**: Type generation - rebuild backend to update frontend types
- **axum**: Web framework with tower middleware stack
- **sqlx**: Present but not actively used (future auth/state persistence)