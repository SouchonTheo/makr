.PHONY: build release fmt fmt-check clippy lint test clean check

build:
	cargo build

release:
	cargo build --release

fmt:
	cargo fmt --all

clippy:
	cargo clippy --all-targets -- -D warnings

fmt-check:
	cargo fmt --all -- --check

lint: fmt clippy

check: fmt-check clippy test

test:
	cargo test

clean:
	cargo clean
