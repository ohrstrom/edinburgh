FROM debian:13-slim AS rs-builder

WORKDIR /demo

RUN > /etc/apt/sources.list

ENV DEBIAN_FRONTEND=noninteractive \
    CARGO_HOME=/root/.cargo \
    RUSTUP_HOME=/root/.rustup \
    PATH="/root/.cargo/bin:$PATH" \
    CARGO_REGISTRIES_CRATES_IO_PROTOCOL=sparse

RUN set -eux && \
    apt-get update && \
    apt-get install -y \
        git \
        curl \
        htop \
        build-essential && \
    curl https://sh.rustup.rs -sSf | sh -s -- -y

# wasm dependencies
RUN cargo install wasm-pack && \
    rustup target add wasm32-unknown-unknown

# shared lib
COPY shared ./shared

# frame forwarder
COPY frame-forwarder ./frame-forwarder

RUN cd frame-forwarder && \
    cargo build --release && \
    cp target/release/edinburgh-frame-forwarder /usr/local/bin/

# directory service
COPY ensemble-directory ./ensemble-directory

RUN cd ensemble-directory && \
    cargo build --release && \
    cp target/release/edinburgh-ensemble-directory /usr/local/bin/

# wasm module
COPY wasm ./wasm

RUN cd wasm && \
    cargo check && \
    wasm-pack build

FROM oven/bun:1 AS bun-builder

WORKDIR /demo

COPY --from=rs-builder /demo/wasm /demo/wasm

COPY web-ui ./web-ui

RUN cd web-ui && \
    bun install && \
    bun run build-only

# runtime stage
FROM debian:13-slim
#FROM bitnami/minideb:bookworm

WORKDIR /demo

RUN set -eux && \
    apt-get update && \
    apt-get install -y --no-install-recommends \
        nginx \
        tini && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

COPY --from=rs-builder /usr/local/bin/edinburgh-frame-forwarder /usr/local/bin/
COPY --from=rs-builder /usr/local/bin/edinburgh-ensemble-directory /usr/local/bin/

COPY --from=bun-builder /demo/web-ui/dist ./dist

COPY \
  demo/docker/start.sh \
  demo/docker/nginx.conf \
  ./

ENTRYPOINT ["/usr/bin/tini", "--"]

CMD ["./start.sh"]
