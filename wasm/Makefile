SHELL := bash
.ONESHELL:
.SHELLFLAGS := -eu -o pipefail -c
.DELETE_ON_ERROR:
MAKEFLAGS += --warn-undefined-variables
MAKEFLAGS += --no-builtin-rules


.PHONY: setup
setup:
	cargo install wasm-pack
	rustup target add wasm32-unknown-unknown

.PHONY: fmt
fmt:
	cargo fmt --all

.PHONY: clippy
clippy:
	cargo clippy --no-deps --all-targets -- -D warnings

.PHONY: clean
clean:
	cargo clean
	rm -Rf pkg

.PHONY: build
build:
	wasm-pack build
