SHELL := bash
.ONESHELL:
.SHELLFLAGS := -eu -o pipefail -c
.DELETE_ON_ERROR:
MAKEFLAGS += --warn-undefined-variables
MAKEFLAGS += --no-builtin-rules

PORT := 3001


.PHONY: fmt
fmt:
	bun run format

.PHONY: lint
check:
	bun run lint

.PHONY: clean
clean:
	cargo clean
	rm -Rf pkg

.PHONY: build
build:
	bun run build

.PHONY: dev
dev:
	bun run dev --port $(PORT)
