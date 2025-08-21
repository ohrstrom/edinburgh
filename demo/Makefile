SHELL := bash
.ONESHELL:
.SHELLFLAGS := -eu -o pipefail -c
.DELETE_ON_ERROR:
MAKEFLAGS += --warn-undefined-variables
MAKEFLAGS += --no-builtin-rules

GCP_ID := rnd-edinburgh
IMAGE := edinburgh-demo:latest
PORT := 3001


.PHONY: build
build:
	docker build -f docker/Dockerfile -t $(IMAGE) ../

.PHONY: run
run:
	docker run --rm -it -p 9000:80 $(IMAGE)


.PHONY: gcp-build
gcp-build:
	docker buildx build --platform linux/amd64 -f docker/Dockerfile -t eu.gcr.io/$(GCP_ID)/$(IMAGE) ../
	docker push eu.gcr.io/$(GCP_ID)/$(IMAGE)


.PHONY: gcp-deploy
gcp-deploy: gcp-build
	gcloud run services update \
	  edinburgh-demo \
	  --image eu.gcr.io/$(GCP_ID)/$(IMAGE) \
	  --region europe-west4