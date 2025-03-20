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

.PHONY: run-cli
run-cli-sync:
	cargo run -p $(CLI_BIN) --bin $(CLI_BIN)-sync -- $(ADDR)

.PHONY: run-frame-forwarder
run-frame-forwarder:
	cargo run -p $(FRAME_FORWARDER_BIN) -- $(ADDR)

.PHONY: run-all
run-all: run-cli run-frame-forwarder

.PHONY: fmt
fmt:
	cargo fmt --all

.PHONY: clippy
clippy:
	cargo clippy --all-targets -- -D warnings

.PHONY: clean
clean:
	cargo clean
