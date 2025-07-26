FROM debian:12

WORKDIR /mux/

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
        tini \
        build-essential \
        automake \
        libtool \
        libzmq3-dev \
        libzmq5 \
        libboost-system-dev \
        libcurl4-openssl-dev && \
    curl https://sh.rustup.rs -sSf | sh -s -- -y

# ODR-DabMux
RUN git clone https://github.com/Opendigitalradio/ODR-DabMux.git
RUN cd ODR-DabMux && \
    ./bootstrap.sh && \
    ./configure && \
    make && \
    make install

# ODR-DabMux-GUI
RUN git clone https://github.com/Opendigitalradio/ODR-DabMux-GUI.git
RUN cd ODR-DabMux-GUI && \
    cargo build --release && \
    cp target/release/odr-dabmux-gui /usr/local/bin/odr-dabmux-gui

COPY \
  mux/start.sh \
  ./

# we use the "hard coded" path of DabMux-GUI here
COPY \
  mux/odr-dabmux.json \
  /etc/odr-dabmux.json


ENTRYPOINT ["/usr/bin/tini", "--"]

CMD ["./start.sh"]