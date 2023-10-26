FROM rust:bookworm AS chef

RUN cargo install cargo-chef --locked

WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
WORKDIR /opt/app
COPY --from=planner app/recipe.json recipe.json

RUN apt update -y \
    && apt upgrade -y \
    && apt install build-essential libsnappy-dev librocksdb-dev libclang-dev clang cmake g++-multilib protobuf-compiler mold -y

RUN rustup target add wasm32-unknown-unknown

RUN cargo chef cook --release --recipe-path recipe.json
RUN cargo chef cook --release --check --recipe-path recipe.json

COPY . .

RUN cargo build --release 

FROM bitnami/minideb:bookworm AS runtime
RUN apt update -y \
    && apt upgrade -y \
    && apt install ca-certificates librocksdb-dev -y \
    && rm -rf /var/lib/apt/lists/* /var/lib/dpkg/* /var/cache/*

EXPOSE 30333 30333/udp 9944 9933

COPY --from=builder /opt/app/target/release/mosaic-chain /usr/local/bin/

CMD ["mosaic-chain"]
