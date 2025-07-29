FROM rust:1.87-slim-bookworm

RUN apt-get update && \
  apt-get install -y --no-install-recommends \
  build-essential \
  cmake \
  jq \
  git \
  gcc \
  curl \
  clang && \
  rm -rf /var/lib/apt/lists/*

RUN rustup target add wasm32-unknown-unknown && \
  rustup component add llvm-tools && \
  rustup component add rustc-dev

RUN curl -L https://sp1up.succinct.xyz | bash && \
  /root/.sp1/bin/sp1up

ADD rust-wrappers.tar.gz /

RUN . /root/.bashrc && \
  cd /rust-wrappers/dummy-circuit && \
  $CARGO_HOME/bin/cargo prove build && \
  cd / && \
  rm -fr /rust-wrappers/dummy-circuit

COPY ./entrypoint.sh /root/entrypoint.sh

WORKDIR /usr/src/app

ENTRYPOINT ["/root/entrypoint.sh"]
