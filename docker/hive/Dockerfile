# trunk-ignore-all(terrascan/AC_DOCKER_0047)

# CairoVM Chain
FROM ghcr.io/dojoengine/dojo:v0.6.0-alpha.2 as katana

# Indexer service
### Apibara DNA indexer and indexer
FROM quay.io/apibara/starknet:1.4.1 as apibara
FROM quay.io/apibara/sink-mongo:0.7.0 as indexer
FROM debian:bookworm as apibara-build
RUN apt-get update && apt-get install -y patchelf && rm -rf /var/lib/apt/lists/*

# Apibara Starknet and Sink are built with Nix, so we need to extract the binaries from the Nix store
# Depending on the platform, the path to the binaries will be different
# These values need to be updated if the version of ApiBara Starknet or Sink change.
# platform: linux/amd64:
#   - APIBARA_STARKNET_BIN_DIR: dsrifc349xgnwsv661shqljxb8pbxx2j
#   - APIBARA_SINK_BIN_DIR: sci7y4a8r7yz05094pl88h3d7vv1s98f
# platform: linux/arm64
#   - APIBARA_STARKNET_BIN_DIR: f7va4mjqww1kkpp4il6y295dgcwq147v
#   - APIBARA_SINK_BIN_DIR: 5p0wgpyfsk19v8rrprnykiv5inzx8zag
ARG APIBARA_STARKNET_BIN_DIR
ARG APIBARA_SINK_BIN_DIR
# Run `docker image inspect apibara/starknet:1.4.1-x86_64` to get the exact path
# ⚠️ This path is subject to change, so it's important to check it before building the image ⚠️
COPY --from=apibara /nix/store/${APIBARA_STARKNET_BIN_DIR}-apibara-starknet-1.4.1/bin/apibara-starknet /usr/local/bin/starknet
COPY --from=indexer /nix/store/${APIBARA_SINK_BIN_DIR}-apibara-sink-mongo-0.7.0/bin/apibara-sink-mongo /usr/local/bin/sink-mongo
# Change the interpreter path.
ARG BUILDPLATFORM
RUN case $BUILDPLATFORM in \
    "linux/amd64") \
    patchelf --set-interpreter /lib64/ld-linux-x86-64.so.2 /usr/local/bin/starknet && \
    patchelf --set-interpreter /lib64/ld-linux-x86-64.so.2 /usr/local/bin/sink-mongo; \
    ;; \
    "linux/arm64") \
    patchelf --set-interpreter /lib/ld-linux-aarch64.so.1 /usr/local/bin/starknet && \
    patchelf --set-interpreter /lib/ld-linux-aarch64.so.1 /usr/local/bin/sink-mongo; \
    ;; \
    *) \
    echo "Unknown BUILDPLATFORM: $BUILDPLATFORM"; \
    exit 1; \
    ;; \
    esac

#### MongoDB
FROM mongo:6.0.8 as mongo

# Ethereum RPC Server
### Build the Cairo programs artifacts
FROM python:3.10.13 as compiler
# install poetry
ENV POETRY_VERSION=1.7.1
ENV PATH="$PATH:/root/.local/bin:/root/.foundry/bin"
ARG GITHUB_TOKEN
ENV GITHUB_TOKEN=$GITHUB_TOKEN

RUN curl -sSL https://install.python-poetry.org | python3 -
RUN poetry config virtualenvs.create false

WORKDIR /usr/src/compiler

# Install dependencies
RUN curl -L https://foundry.paradigm.xyz -o foundry.sh \
    && chmod +x foundry.sh \
    && ./foundry.sh \
    && foundryup \
    && apt-get update && apt-get install -y \
    jq \
    wget \
    tar \
    unzip \
    zip

COPY . .

RUN make setup

COPY . .

### Build the RPC server
# Define ARG for build platform
FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json

# Build application
COPY . .
# Install system dependencies
RUN apt-get update && \
    apt-get -y upgrade && \
    apt-get install -y gcc-aarch64-linux-gnu libssl-dev clang libclang-dev

# Set working directory
WORKDIR /usr/src/rpc

COPY . .

COPY --from=compiler /usr/src/compiler/.kakarot/artifacts /usr/src/rpc/.kakarot/artifacts


RUN cargo build \
    --features hive --release --target-dir /usr/src/rpc/target && \
    cargo build \
    --bin hive_genesis --release --features testing --target-dir /usr/src/rpc/target && \
    cargo build \
    --bin hive_chain --release --features testing --target-dir /usr/src/rpc/target

FROM debian:bookworm-slim as base
# Install any necessary dependencies
RUN apt-get update && apt-get install -y tini curl jq libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*

# Environment
#############
### Indexer environment variables
#### Indexer environment variables
ENV ALLOW_ENV_FROM_ENV=DEBUG,APIBARA_AUTH_TOKEN,STARTING_BLOCK,STREAM_URL,SINK_TYPE,MONGO_CONNECTION_STRING,MONGO_DATABASE_NAME,STARKNET_NETWORK,KAKAROT_ADDRESS,ALLOW_NET
ENV DEBUG=""
ENV APIBARA_AUTH_TOKEN=""
ENV MONGO_CONNECTION_STRING=mongodb://localhost:27017
ENV MONGO_DATABASE_NAME=kakarot-local
ENV STARTING_BLOCK=0
ENV STREAM_URL=http://localhost:7171
ENV SINK_TYPE=mongo
ENV KATANA_ACCOUNT_ADDRESS=0xb3ff441a68610b30fd5e2abbf3a1548eb6ba6f3559f2862bf2dc757e5828ca
ENV KATANA_PRIVATE_KEY=0x2bbf4f9fd0bbb2e60b0316c1fe0b76cf7a4d0198bd493ced9b8df2a3a24d68a
ENV ALLOW_NET=""
### Kakarot RPC environment variables
### Port 8545: https://github.com/ethereum/hive/blob/master/simulators/ethereum/rpc/helper.go#L50
ENV KAKAROT_RPC_URL=0.0.0.0:8545
ENV STARKNET_NETWORK=http://localhost:5050
ENV RUST_LOG=kakarot_rpc=info

HEALTHCHECK --interval=10s --timeout=10s --start-period=15s --retries=5 \
    CMD response=$(curl --silent --request POST \
    --header "Content-Type: application/json" \
    --data '{"jsonrpc": "2.0", "method": "eth_getBlockByNumber", "params": ["latest", true], "id": 1}' \
    http://${KAKAROT_RPC_URL} | jq -e '.result != null') && echo $response && [ "$response" = "true" ] || exit 1


# Ports
#######
# 8545 Ethereum RPC
# 27017 MongoDB
EXPOSE 8545 27017

# Copy binaries and dependencies
################################
### CairoVM chain service
COPY --from=katana /usr/local/bin/katana /usr/local/bin

### Indexer service
#### We need the DNA indexer binary
COPY --from=apibara-build /usr/local/bin/starknet /usr/local/bin/starknet

#### We need the indexer typescript code and the binary that knows how to run it
COPY ./indexer /usr/src/app/code/indexer
COPY --from=apibara-build /usr/local/bin/sink-mongo /usr/local/bin/sink-mongo

#### We need the mongo binary
COPY --from=mongo /bin/mongod /usr/local/bin
RUN mkdir -p /data/db


# Generate the genesis
COPY --from=builder /usr/src/rpc/target/release/kakarot-rpc /usr/local/bin/kakarot-rpc
COPY --from=builder /usr/src/rpc/target/release/hive_genesis /usr/local/bin/hive_genesis
COPY --from=builder /usr/src/rpc/target/release/hive_chain /usr/local/bin/hive_chain
RUN mkdir -p /genesis/contracts
COPY --from=compiler /usr/src/compiler/.kakarot/build /genesis/contracts

# Copy start script
COPY docker/hive/start.sh /start.sh
RUN chmod +x /start.sh

ENTRYPOINT ["/usr/bin/tini", "--", "/start.sh"]

CMD []
