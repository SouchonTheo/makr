# makr

A fast, interactive TUI for exploring and running Makefile targets.

![makr demo](assets/demo.gif)

[![CI](https://github.com/SouchonTheo/makr/actions/workflows/ci.yml/badge.svg)](https://github.com/SouchonTheo/makr/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/makr.svg)](https://crates.io/crates/makr)
[![Downloads](https://img.shields.io/crates/d/makr.svg)](https://crates.io/crates/makr)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
![Rust](https://img.shields.io/badge/rust-2024-orange)

## Features

- **Browse targets** — lists all targets from your Makefile in a navigable panel, with `.PHONY` indicators
- **Target details** — shows dependencies, commands, and used variables for each target, with scrollable detail view
- **Fuzzy search** — quickly find targets with `/`, with matched characters highlighted and results ranked by match quality
- **Variable editor** — edit Makefile variables before running a target, with a live command preview
- **Dry-run mode** — preview what `make` would do without executing (toggle with `Ctrl+n` in popup, or `--dry-run` flag)
- **Include support** — follows `include`, `-include`, and `sinclude` directives, with glob expansion (`*.mk`)
- **Run targets** — execute any target directly from the TUI with execution result displayed in the status bar
- **Robust parser** — handles continuation lines (`\`), `export`/`override` prefixes, `define` blocks, and all assignment types (`=`, `:=`, `?=`, `+=`)

## Installation

From [crates.io](https://crates.io/crates/makr):

```sh
cargo install makr
```

From source:

```sh
git clone https://github.com/SouchonTheo/makr
cd makr
cargo install --path .
```

## Usage

```
makr [OPTIONS] [FILE]
```

**Arguments:**

- `[FILE]` — Path to a Makefile (default: `./Makefile`)

**Options:**

- `-n, --dry-run` — Start in dry-run mode (pass `-n` to make)
- `-h, --help` — Show help message
- `-v, --version` — Show version

### Examples

```sh
# Explore the default Makefile
makr

# Explore a specific Makefile
makr path/to/Makefile

# Start in dry-run mode
makr --dry-run
```

## Keybindings

### Normal mode

| Key              | Action                |
|------------------|-----------------------|
| `j` / `k`, `↑` / `↓` | Navigate targets |
| `Ctrl+d` / `Ctrl+u`   | Scroll detail panel |
| `/`              | Open fuzzy search     |
| `Enter`          | Run selected target   |
| `q`, `Esc`       | Quit                  |

### Search mode

| Key              | Action                |
|------------------|-----------------------|
| `↑` / `↓`       | Navigate results      |
| `Enter`          | Select target         |
| `Esc`            | Cancel search         |

### Run popup

| Key              | Action                    |
|------------------|---------------------------|
| `Tab` / `↑` / `↓` | Switch between variables |
| `Ctrl+n`         | Toggle dry-run mode       |
| `Enter`          | Execute target            |
| `Esc`            | Cancel                    |

## How it works

1. **Parsing** — makr parses the Makefile (and included files) to extract variables, targets, dependencies, and commands. It handles continuation lines, `define` blocks, `export`/`override` prefixes, and all variable assignment types.
2. **Browsing** — Targets are displayed in a left panel with `.PHONY` tags; selecting one shows its details (dependencies, commands, variables) on the right. The detail panel is scrollable with `Ctrl+d`/`Ctrl+u`.
3. **Searching** — Press `/` to fuzzy-search targets. Results are ranked by match quality (prefix matches first, then consecutive characters, then spread-out matches).
4. **Running** — Press `Enter` on a target to open a popup where you can edit the relevant variables, toggle dry-run mode, and see a live preview of the command. makr then shells out to `make` with your overrides and displays the result in the status bar.

## Dependencies

- [ratatui](https://github.com/ratatui/ratatui) — Terminal UI framework
- [crossterm](https://github.com/crossterm-rs/crossterm) — Terminal manipulation

## Contributing

Contributions are welcome! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

MIT
