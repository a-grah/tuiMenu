# tuimenu

A configurable command-menu TUI written in Rust. Pick a shell command from a
searchable list and run it — then return to the menu. Designed as a replacement
for a `pmenu`-based workflow.

## Features

- **Vi-style modal keys** — `j`/`k` navigation, `/` to search, `o`/`c`/`dd` to manage entries
- **Fuzzy filtering** — type to narrow by name or description (word-ranked, like pmenu)
- **Non-selectable headings** — organise commands into labelled sections
- **In-TUI editing** — add, edit, and delete entries without touching the config file
- **TOML config** at `~/.tuimenu.toml`, seeded with a starter menu on first run
- **Widget row** — used memory, live BTC/USD price, and weather (via `wttr.in`)
- **Per-widget refresh intervals** — all configurable, default 5 minutes

## Installation

Requires Rust. If not installed:

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
. "$HOME/.cargo/env"
```

Build and run:

```sh
git clone <repo-url>
cd tuiMenu
cargo build --release
./target/release/tuimenu
```

For convenience, copy the binary somewhere on your PATH:

```sh
cp target/release/tuimenu /usr/local/bin/
```

## Key bindings

| Key | Action |
|-----|--------|
| `j` / `k`, `↓` / `↑` | Move selection (skips headings) |
| `gg` / `G` | First / last entry |
| `Ctrl-d` / `Ctrl-u` | Half page down / up |
| `/` | Enter search mode (live filter) |
| `Enter` / `l` | Run selected command |
| `o` | New entry (inserted after selection) |
| `c` | Edit selected entry |
| `dd` | Delete selected entry |
| `?` | Help overlay |
| `q` / `:q` | Quit |

**Form mode** (add/edit): `j`/`k`/`Tab` move between fields, `i`/`Enter` edit a
field, `Esc` stop editing, `Ctrl-s` or `ZZ` save, `q`/`Esc` cancel.

**Confirm delete**: `y` to confirm, `n` or `Esc` to cancel.

## Configuration

The config file is created at `~/.tuimenu.toml` on first run. Edit it directly
or use the in-TUI commands.

```toml
[settings]
weather_location     = "Oakville"
weather_format       = "2"          # wttr.in ?format=
weather_refresh_secs = 300
memory_refresh_secs  = 300
btc_refresh_secs     = 300
btc_price_url        = "https://api.coingecko.com/api/v3/simple/price?ids=bitcoin&vs_currencies=usd"

[[entries]]
heading = "System"

[[entries]]
name        = "Disk free"
description = "Free disk space"
command     = "df -h"
```

Each `[[entries]]` block is either a **heading** (`heading = "Title"`) or a
**command** (`name`, `description`, `command`). After editing the file manually,
restart the app to pick up the changes.

## Version management

The version shown in the `?` help overlay is read from `Cargo.toml` at compile
time. Git tags and `Cargo.toml` are kept in sync manually — this is the standard
Rust convention.

To release a new version:

1. Bump `version` in `Cargo.toml`:
   ```toml
   [package]
   version = "0.0.2"
   ```
2. Build so the new version is baked into the binary:
   ```sh
   cargo build --release
   ```
3. Commit and tag:
   ```sh
   git add Cargo.toml Cargo.lock
   git commit -m "Release 0.0.2"
   git tag -a v0.0.2 -m "Version 0.0.2"
   ```

## License

This project is released into the public domain under [The Unlicense](LICENSE).
You are free to use, copy, modify, and distribute it for any purpose, with no conditions.

## Smoke test

Run without a terminal (useful for CI or verifying widget connectivity):

```sh
tuimenu --check
```

Prints the config path, entry count, hostname, memory, BTC price, and weather.
