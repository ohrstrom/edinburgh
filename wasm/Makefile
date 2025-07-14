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
	rm -Rf pkg

.PHONY: build
build:
	wasm-pack build
