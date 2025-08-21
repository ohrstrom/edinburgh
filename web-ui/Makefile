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
	#bun run type-check

.PHONY: clean
clean:
	cargo clean
	rm -Rf pkg

.PHONY: build
build:
	bun run build-only

.PHONY: preview
preview: build
	bun run preview --port $(PORT)

.PHONY: dev
dev:
	bun run dev --port $(PORT)

.PHONY: dev-gcp
dev-gcp:
	VITE_FRAME_FORWARDER_ENDPOINT=wss://edinburgh-demo-845954871796.europe-west4.run.app/ws \
	VITE_ENSEMBLE_DIRECTORY_ENDPOINT=https://edinburgh-demo-845954871796.europe-west4.run.app/ensembles \
	bun run dev --port $(PORT)
