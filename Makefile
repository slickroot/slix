.DEFAULT_GOAL := build

.PHONY: build release fmt clippy test test-chrome

build:
	nix develop --command cargo build

release:
	nix develop --command cargo build --release
	mkdir -p ~/.local/bin && cp target/release/slix ~/.local/bin/slix

fmt:
	nix develop --command cargo fmt

clippy:
	nix develop --command cargo clippy -- -D warnings

test:
	nix develop --command cargo test
	nix develop --command node --test "chrome/*.test.js"

test-chrome:
	nix develop --command node --test "chrome/*.test.js"
