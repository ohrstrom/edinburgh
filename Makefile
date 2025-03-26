SHELL := bash
.ONESHELL:
.SHELLFLAGS := -eu -o pipefail -c
.DELETE_ON_ERROR:
MAKEFLAGS += --warn-undefined-variables
MAKEFLAGS += --no-builtin-rules

CLI_BIN := edinburgh
FRAME_FORWARDER_BIN := edinburgh-frame-forwarder

ADDR ?= 213.232.205.101:8855

.PHONY: run-cli
run-cli:
	cargo run -p $(CLI_BIN) -- --addr $(ADDR)

.PHONY: run-frame-forwarder
run-frame-forwarder:
	cargo run -p $(FRAME_FORWARDER_BIN) -- $(ADDR)

.PHONY: fmt
fmt:
	cargo fmt --all

.PHONY: clippy
clippy:
	cargo clippy --all-targets -- -D warnings

.PHONY: clean
clean:
	cargo clean

.PHONY: build
build:
	cargo build --package edinburgh-wasm --target wasm32-unknown-unknown
	cargo build --package edinburgh
	cargo build --package edinburgh-frame-forwarder

.PHONY: release
release:
	cargo build --release --package edinburgh-wasm --target wasm32-unknown-unknown
	cargo build --release --package edinburgh
	cargo build --release --package edinburgh-frame-forwarder
