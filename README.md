# goto

Navigate to projects using namespace-based paths.

## Why

`goto` removes repetitive `cd` patterns when you jump between many repos and workspaces.

## Features

- Namespace-based navigation (`gh/project`, `work/app`)
- Alias support (`github/project`)
- Dynamic tab completion for targets
- Single static Rust binary
- Simple standalone config model

## Install

### Build/install from source

```bash
cargo install --path .
```

### Verify

```bash
goto --version
```

## Usage

```bash
# Print resolved path for a target
goto gh/project

# Print child entries under namespaces
goto list
goto list gh

# Show active config file path
goto config-path

# Run diagnostics for config + shell setup
goto doctor

# Manage namespaces from CLI
goto add work ~/Projects --alias office
goto rename work client
goto set-path client ~/Code/Client
goto alias-add gh github
goto alias-remove gh github
goto remove work
goto remove github # remove by alias also works

# Print effective config TOML
goto list-raw
```

Without shell integration, `goto` prints the resolved path to stdout.

## Shell integration (zsh)

Run once:

```bash
goto setup
```

This appends a managed helper block to `~/.zshrc` so `goto gh/project` changes the current shell directory and supports tab completion.

Remove integration:

```bash
goto uninstall
```

## Configuration

Find the active config path:

```bash
goto config-path
```

If no config file exists, `goto` uses the built-in default from `default_config.toml`.

Example config:

```toml
[[namespace]]
name = "gh"
path = "~/Documents/GitHub"
aliases = ["github"]

[[namespace]]
name = "work"
path = "~/Projects"
aliases = ["projects"]
```

Rules:

- `name` is required and case-insensitive at lookup time.
- `path` supports shell expansion (`~`, `$HOME`).
- `aliases` are optional and case-insensitive.

## Command reference

- `goto <namespace>/<path>`: resolve and print target path
- `goto list [namespace]`: list namespace roots / children
- `goto setup`: install zsh shell integration
- `goto uninstall`: remove zsh shell integration
- `goto config-path`: print active config file location
- `goto doctor`: run config and setup diagnostics
- `goto add <name> <path> [--alias ...]`: add namespace
- `goto remove <name-or-alias>`: remove namespace
- `goto rename <old> <new>`: rename namespace
- `goto set-path <name> <path>`: update namespace root path
- `goto alias-add <name> <alias>`: add alias
- `goto alias-remove <name> <alias>`: remove alias
- `goto list-raw`: print effective config as TOML

## Architecture

- `src/main.rs`: command dispatch and command handlers
- `src/cli.rs`: clap command definitions and help
- `src/config.rs`: config path resolution and load/save
- `src/namespace.rs`: namespace validation, lookup, completion
- `src/shell.rs`: zsh integration setup/uninstall

## Development

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

Useful local checks:

```bash
cargo run -- doctor
cargo run -- list-raw
```

## License

MIT
