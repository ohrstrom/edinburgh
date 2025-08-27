SHELL := bash
.ONESHELL:
.SHELLFLAGS := -eu -o pipefail -c
.DELETE_ON_ERROR:
MAKEFLAGS += --warn-undefined-variables
MAKEFLAGS += --no-builtin-rules


.PHONY: fmt
fmt:
	cargo fmt --all

.PHONY: clippy
clippy:
	cargo clippy --no-deps --all-targets -- -D warnings

.PHONY: clean
clean:
	cargo clean

.PHONY: build
build:
	cargo build

.PHONY: build-release
build-release:
	cargo build --release
