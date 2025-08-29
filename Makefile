SHELL := bash
.ONESHELL:
.SHELLFLAGS := -eu -o pipefail -c
.DELETE_ON_ERROR:
MAKEFLAGS += --warn-undefined-variables
MAKEFLAGS += --no-builtin-rules


.PHONY: fmt
fmt:
	cargo fmt --all

.PHONY: clean
clean:
	cargo clean

.PHONY: clippy
clippy:
	cargo clippy \
	  --workspace \
	  --exclude edinburgh-wasm \
	  --exclude edinburgh-pyo3 \
	  -- -D warnings
	cargo clippy \
	  --package edinburgh-wasm \
	  --target wasm32-unknown-unknown \
	  --no-deps \
	  -- -D warnings

.PHONY: check
check:
	cargo check \
	  --workspace \
	  --exclude edinburgh-wasm \
	  --exclude edinburgh-pyo3
	cargo check \
	  --package edinburgh-wasm \
	  --target wasm32-unknown-unknown

.PHONY: build
build:
	cargo build \
	  --workspace \
	  --exclude edinburgh-wasm \
	  --exclude edinburgh-pyo3
	cargo build \
	  --package edinburgh-wasm \
	  --target wasm32-unknown-unknown

.PHONY: build-release
build-release:
	cargo build \
	  --release \
	  --workspace \
	  --exclude edinburgh-wasm \
	  --exclude edinburgh-pyo3
	cargo build \
	  --release \
	  --package edinburgh-wasm \
	  --target wasm32-unknown-unknown

.PHONY: install
install:
	cargo install --path cli
	cargo install --path frame-forwarder
	cargo install --path ensemble-directory
