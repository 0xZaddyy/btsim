# Repository Guidelines

## Project Structure & Module Organization
- `src/` contains all Rust sources. The core simulation logic lives in `src/lib.rs`, while the CLI entry point is `src/main.rs`.
- Domain modules are split by concern, such as `src/transaction.rs`, `src/wallet.rs`, and `src/economic_graph.rs`.
- Example configuration lives at `config.toml.example`; the running config is read from `config.toml` by default.
- Generated artifacts include `graph.svg` produced after a simulation run.

## Build, Test, and Development Commands
- `cargo build` compiles the library and binary.
- `cargo run` runs the simulation using `config.toml` (or set `CONFIG_FILE=path/to/config.toml`).
- `cargo test` runs unit tests embedded in the source files.
- Optional: `cargo fmt` applies standard Rust formatting if available.

## Coding Style & Naming Conventions
- Use Rust 2021 edition conventions with 4-space indentation.
- Naming: `snake_case` for functions/variables, `CamelCase` for types/traits, and `SCREAMING_SNAKE_CASE` for constants.
- Keep modules focused; prefer adding new files in `src/` rather than growing monolithic modules.

## Testing Guidelines
- Tests are defined inline in source files with `#[test]` functions.
- Place module-specific tests near the code they cover (e.g., `src/actions.rs` and `src/lib.rs` already contain tests).
- Run `cargo test` before submitting changes; add tests for new behaviors or bug fixes.

## Commit & Pull Request Guidelines
- Commit messages follow short, imperative summaries (e.g., “Add unit tests for payment strategies”).
- PRs should include a concise description, steps to validate (`cargo test` or `cargo run`), and any relevant output artifacts (e.g., updated `graph.svg` when behavior changes).

## Configuration & Outputs
- Copy `config.toml.example` to `config.toml` and customize settings such as `simulation.seed` and `wallet_types`.
- The binary writes a transaction graph to `graph.svg`; include it in reviews when output structure changes.
