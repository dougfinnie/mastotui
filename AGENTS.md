# AGENTS.md

## Cursor Cloud specific instructions

### Overview

**mastotui** is a single-binary Rust TUI client for Mastodon. No backend services, databases, or Docker containers are needed — it's a pure client-side app that talks to external Mastodon instances via HTTPS.

### System dependencies

The `keyring` crate requires D-Bus development headers on Linux. The update script handles this automatically (`sudo apt-get install -y libdbus-1-dev pkg-config`).

### Rust toolchain

The project requires Rust edition 2024 support (via `ratatui-widgets` 0.3.0), which means **Rust 1.85+**. The VM's pre-installed Rust may be older; the update script runs `rustup default stable` and `rustup update stable` to ensure the latest stable toolchain is active.

### Build / Lint / Test

Standard commands — see `README.md` and `.github/workflows/ci.yml` for the canonical CI pipeline:

- `cargo fmt -- --check` — format check
- `cargo clippy -- -D warnings` — lint (uses pedantic + nursery lints)
- `cargo test` — 25 unit + integration tests
- `cargo build --release` — optimized binary

### Known caveats

- **Clippy `use_self` lint**: On Rust 1.93+ the `clippy::use_self` lint fires on `Status.reblog` in `src/api/types.rs`. This is a pre-existing upstream issue — the CI may pin to an older stable that doesn't trigger it. Do not modify the code to fix it unless the repo owner updates.
- **TUI requires a real TTY**: `cargo run` needs an interactive terminal (crossterm + stdout). In headless/shell-only contexts, it will error. Use the Desktop pane terminal or `dbus-run-session` wrapper when needed.
- **Mastodon credentials**: Full end-to-end testing (OAuth login, posting, timeline) requires a real Mastodon account. Unit/integration tests do **not** require credentials.
- **Tracey** (spec traceability tool) is optional. It's used by `./scripts/check-spec.sh` but not required for `cargo build`/`cargo test`.
