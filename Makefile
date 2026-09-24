.DEFAULT_GOAL := build

.PHONY: build release fmt clippy

build:
	nix develop --command cargo build

release:
	nix develop --command cargo build --release
	mkdir -p ~/.local/bin && cp target/release/slix ~/.local/bin/slix

fmt:
	nix develop --command cargo fmt

clippy:
	nix develop --command cargo clippy -- -D warnings
