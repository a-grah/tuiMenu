# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Overview

`tuimenu` is a configurable command-menu TUI in Rust (ratatui + crossterm). It presents a searchable, vi-keyed list of shell commands; selecting one drops out of the TUI, runs the command in the user's `$SHELL`, then returns. A status row shows live widgets (memory, BTC price, weather, uptime).

## Commands

```sh
cargo build                 # debug build
cargo build --release        # release build -> target/release/tuimenu
cargo test                   # run all tests (unit tests live in src/app.rs)
cargo test navigation_skips_headings   # run a single test by name
cargo run                    # build + launch the TUI
cargo run -- --check         # non-interactive smoke test (see below)
```

Rust must be on PATH; if `cargo` isn't found, `source "$HOME/.cargo/env"` first.

### `--check` smoke test

`tuimenu --check` validates the config and fetches every widget once (memory, BTC, weather), printing results without entering raw mode. Use it to verify config parsing and network-widget connectivity in CI or over SSH where a TUI can't run.

## Architecture

Four modules under `src/`, with a deliberate separation between state, rendering, and the terminal lifecycle:

- **`main.rs`** — terminal lifecycle and the event loop. `run_loop` ticks widgets, draws, then `event::poll`s with a 250 ms timeout (so the UI redraws ~4×/sec even when idle, e.g. to refresh widgets). The key inversion-of-control detail: **running a command happens in `main`, not `App`.** `App::activate` only sets `app.run_request`; `run_loop` detects it, tears down the terminal (`restore_terminal`), runs the command via `run_command_blocking`, then rebuilds the terminal. This is why command execution can't live inside the key handlers.

- **`app.rs`** — all application state (`App`) and input handling; no rendering, no I/O beyond config persistence. Input is a modal state machine over `Mode` (Normal / Search / Command / Form / Confirm), each with its own `on_key_*` handler dispatched from `on_key`. Two-key vi sequences (`gg`, `dd`, `ZZ`) are handled via the `pending: Option<char>` field. Unit tests at the bottom drive the app by feeding synthetic `KeyEvent`s through `on_key` and asserting on state — this is the primary test strategy; follow it for new behavior.

- **`config.rs`** — TOML model and persistence at `~/.tuimenu.toml`. A single `Entry` struct represents **both** headings and commands; all fields are `Option` so serde round-trips either shape, and `is_heading`/`is_runnable` discriminate at runtime. `Config::load` seeds a default file (`Config::seed`) on first run. Mutating actions in `app.rs` call `persist()` immediately, so the config file is always in sync with in-memory state.

- **`widgets.rs`** — status-row data. Two refresh strategies coexist: **network widgets** (BTC, weather) run on background threads via `spawn_poller`, writing into `Arc<Mutex<Option<String>>>` slots read during render; **local widgets** are pulled synchronously — memory is gated by an interval inside `Widgets::tick()`, while uptime is recomputed every frame (cheap, no gating). All network fetches go through `curl` as a subprocess (no HTTP crate dependency).

### Key model invariants (in `app.rs`)

- **Selection always lands on a runnable entry.** Headings are non-selectable; `move_selection`, `ensure_valid_selection`, and `go_edge` skip over them. When changing filtering/navigation logic, preserve this — the tests assert it.
- **`filtered: Vec<usize>`** holds indices into `config.entries`; `selected` indexes into `filtered`. Search ranks matches in three tiers (exact word / prefix / substring) in `recompute`.

## Releasing

Versioning is manual (standard Rust convention): bump `version` in `Cargo.toml`, `cargo build` to bake it into the binary (the `?` help overlay reads it via `env!("CARGO_PKG_VERSION")` at compile time), commit, then tag `vX.Y.Z`. Pushing a `v*.*.*` tag triggers `.github/workflows/release.yml`, which cross-compiles macOS (arm64 + x86_64) and Linux (x86_64 + arm64, static musl via cargo-zigbuild) binaries and attaches them to a GitHub release. Keep `Cargo.toml` and the git tag in sync.
