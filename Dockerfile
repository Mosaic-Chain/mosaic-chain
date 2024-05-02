FROM rust:bookworm AS builder

WORKDIR /app

RUN apt update -y \
    && apt upgrade -y \
    && apt install build-essential libsnappy-dev librocksdb-dev libclang-dev clang cmake g++-multilib protobuf-compiler -y

RUN rustup target add wasm32-unknown-unknown

COPY . .

RUN cargo build --release

FROM bitnami/minideb:bookworm AS runtime
RUN apt update -y \
    && apt upgrade -y \
    && apt install ca-certificates librocksdb-dev -y \
    && rm -rf /var/lib/apt/lists/* /var/lib/dpkg/* /var/cache/*

EXPOSE 30333 30333/udp 9944 9933

COPY --from=builder /app/target/release/mosaic-testnet-solo /usr/local/bin/

CMD ["mosaic-testnet-solo"]
