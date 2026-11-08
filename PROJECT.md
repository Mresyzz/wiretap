# Project: WireTap Architecture & Micro-UX Modernization

## Architecture
- **Core Proxy**: `hudsucker` MITM HTTP/HTTPS proxy library (Tokio-native, non-blocking callback interceptor), strictly replacing/upgrading custom proxy without raw `tokio::net::TcpStream`.
- **Web Server & Embedded Assets**: `axum 0.7` HTTP server combined with `rust-embed` to compile and bundle static HTML/CSS/JS directly into a single binary executable.
- **Realtime Push**: Axum WebSocket endpoint (`/api/ws`) relaying packets captured in `TrafficStore` via `tokio::sync::broadcast::channel` to connected web clients.
- **Frontend & UX**: Single-file HTML in `src/web/ui.rs` / embedded assets, zero npm build steps. Enhanced with ARIA accessibility tags, keyboard navigation, `:focus-visible` states, and micro-UX interactions strictly within 50 lines of code.

## Feature Inventory
| # | Feature | Description | Milestone | Source |
|---|---------|-------------|-----------|--------|
| 1 | Dependency Modernization | Add `hudsucker`, `rust-embed`, enable `axum` feature `ws` in Cargo.toml | M1 | Survey 1, 2 |
| 2 | WebSocket Realtime Push | Axum WebSocket handler (`/api/ws`) streaming `TrafficEntry` from `broadcast::channel` | M1 | Survey 1, 2 |
| 3 | Static Asset Embedding via rust-embed | Use `rust-embed` to embed and serve frontend assets directly from binary | M1 | Survey 1, 2 |
| 4 | Hudsucker MITM Proxy Core | Integrate `hudsucker::HttpHandler` to intercept requests/responses without raw `TcpStream` | M1 | Survey 1, 2 |
| 5 | Palette Micro-UX & ARIA Enhancements | In `src/web/ui.rs`: add ARIA tags, `:focus-visible` CSS, keyboard stream item navigation, `/` search hotkey (≤50 lines) | M2 | Survey 3 |
| 6 | Palette UX Journal (`.Jules/palette.md`) | Create `.Jules/palette.md` with standardized UX/a11y learnings | M2 | Survey 3 |
| 7 | Bug Finder Adversarial & Regression Testing | Adversarial testing of proxy streaming, WebSocket resilience, UI a11y, cargo test verification | M3 | R2 |
| 8 | Forensic Integrity Audit & Release Build | Independent audit for cheating/facades; verify `cargo check`, `cargo test`, `cargo build --release` | M4 | System Policy & Acceptance Criteria |

## Milestones
| # | Name | Scope | Dependencies | Status |
|---|------|-------|-------------|--------|
| 1 | Backend Architecture Compliance | Cargo.toml dependencies, `rust-embed` asset embedding, Axum WebSocket handler, `hudsucker` proxy integration | none | DONE (7b5c2e62) |
| 2 | Palette Micro-UX & Accessibility | `src/web/ui.rs` & `static/index.html` micro-UX (<=50 lines), ARIA tags, `:focus-visible`, keyboard navigation, `.Jules/palette.md` | M1 | DONE (7afa90cd) |
| 3 | Bug Finder Review & Boundary Testing | Comprehensive defect review, boundary tests, WebSocket & proxy regression testing | M1, M2 | DONE (e9c3109e) |
| 4 | Forensic Audit & Release Build | Forensic integrity audit, `cargo check`, `cargo test`, `cargo build --release` | M1, M2, M3 | DONE (decb3388) |

## Interface Contracts
### `TrafficStore` ↔ Axum WebSocket (`/api/ws`)
- `TrafficStore::subscribe(&self) -> broadcast::Receiver<TrafficEntry>`
- WebSocket handler: upgrades connection, listens on receiver, serializes `TrafficEntry` to JSON, and sends text messages to client.
- Forward compatibility: keep `/api/events` (SSE) or gracefully migrate frontend to WebSocket.

### Static Assets ↔ Web Server
- `rust_embed::RustEmbed` struct: binds asset directory or file.
- `index_handler`: retrieves asset from `RustEmbed` and serves as `Html`.

### Palette Micro-UX ↔ `src/web/ui.rs`
- Maximum line delta: <= 50 lines.
- Accessibility standards: WCAG 2.1 AA focus-visible and ARIA role/attribute guidelines.
- Hotkeys: `/` to focus filter input, `Escape` to blur.

## Code Layout
- `Cargo.toml`: dependency definitions (`hudsucker`, `rust-embed`, `axum` with `ws`).
- `src/proxy.rs` & `src/main.rs`: proxy engine and CLI wiring.
- `src/web/mod.rs`: Axum router, WebSocket handler, and asset serving.
- `src/web/ui.rs`: single-file HTML/CSS/JS UI definition.
- `src/store.rs`: `TrafficStore` with `tokio::sync::broadcast::channel`.
- `.Jules/palette.md`: Palette UX and accessibility learning journal.
- `tests/unit_test.rs`: regression test suite.
