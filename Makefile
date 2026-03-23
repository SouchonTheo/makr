.PHONY: build release fmt clippy lint test clean check

build:
	cargo build

release:
	cargo build --release

fmt:
	cargo fmt --all

clippy:
	cargo clippy -- -D warnings

lint: fmt clippy

test:
	cargo test

check: lint test

clean:
	cargo clean
