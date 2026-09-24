# 008 - Makefile for development scripts

## Technical Design

Purely technical — no user story. Adds a root `Makefile` wrapping the common
cargo workflows through the project's nix devShell, plus a `flake.nix` update
so `clippy` is available to run.

### `flake.nix`

- Add `clippy` to the `devShells.default` package list (alongside `cargo`,
  `rustc`, `rustfmt`), since `make clippy` needs it.

### `Makefile` (new, repo root)

- `.DEFAULT_GOAL := build`, so bare `make` and `make build` both run the same
  target.
- `build`: `nix develop --command cargo build`.
- `release`: `nix develop --command cargo build --release`, then
  `mkdir -p ~/.local/bin && cp target/release/slix ~/.local/bin/slix`. The
  copy step runs as plain shell (no nix needed) since it's just a file copy.
- `fmt`: `nix develop --command cargo fmt` — rewrites files in place.
- `clippy`: `nix develop --command cargo clippy -- -D warnings` — fails the
  build on any warning; zero warnings is the standing bar.
- All targets declared `.PHONY` (`build`, `release`, `fmt`, `clippy`).
