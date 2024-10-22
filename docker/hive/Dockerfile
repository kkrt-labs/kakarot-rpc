# trunk-ignore-all(terrascan/AC_DOCKER_0047)

# CairoVM Chain
FROM ghcr.io/dojoengine/dojo:v1.0.0-alpha.14 as katana

# Indexer service
### Apibara DNA indexer and indexer
FROM quay.io/apibara/starknet:1.6.0 as apibara
FROM quay.io/apibara/sink-mongo:0.9.2 as indexer
FROM debian:bookworm as apibara-build
RUN apt-get update && apt-get install --no-install-recommends -y patchelf && rm -rf /var/lib/apt/lists/*

# Apibara Starknet and Sink are built with Nix, so we need to extract the binaries from the Nix store
# ⚠️ When modifying the tag of Apibara, make sure to change the Github CI workflow, and Indexer dockerfile as well
# Depending on the platform, the path to the binaries will be different
# These values need to be updated if the version of ApiBara Starknet or Sink change.
# platform: linux/amd64:
#   - APIBARA_STARKNET_BIN_DIR: 3l93dydg7m71r66x5wllf1j9czvj7zdk
#   - APIBARA_SINK_BIN_DIR: 8z122nm3b42lgknvv5zp7h921n3f6ia6
# platform: linux/arm64
#   - APIBARA_STARKNET_BIN_DIR: ksmcmb1ybrij98hjy4q54v31ag7d2l6l
#   - APIBARA_SINK_BIN_DIR: g30fi76vhnamm9wiaqnynnckbhvwdyg1
ARG APIBARA_STARKNET_BIN_DIR
ARG APIBARA_SINK_BIN_DIR
# Run `docker image inspect apibara/starknet:1.6.0-x86_64` to get the exact path
# Run `docker image inspect apibara/starknet:1.6.0-aarch64` to get the exact path
# ⚠️ This path is subject to change, so it's important to check it before building the image ⚠️
COPY --from=apibara /nix/store/${APIBARA_STARKNET_BIN_DIR}-apibara-starknet-1.6.0/bin/apibara-starknet /usr/local/bin/starknet
COPY --from=indexer /nix/store/${APIBARA_SINK_BIN_DIR}-apibara-sink-mongo-0.9.2/bin/apibara-sink-mongo /usr/local/bin/sink-mongo
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
ENV PATH="$PATH:/root/.local/bin:/root/.foundry/bin:/root/.cargo/bin"

# Install dependencies
RUN --mount=type=cache,target=/root/.cache curl -L https://foundry.paradigm.xyz -o foundry.sh \
    && curl -LsSf https://astral.sh/uv/install.sh | sh \
    && chmod +x foundry.sh \
    && ./foundry.sh \
    && foundryup \
    && apt-get update && apt-get install --no-install-recommends -y \
    jq \
    wget \
    docker.io \
    tar \
    unzip \
    zip

WORKDIR /usr/src/compiler

COPY . .

# Similar to `make setup` but we remove the `make build-sol` step
# which uses docker to build the experimental solidity contracts.
# Using docker in docker is not supported in the current setup.

# Install asdf for multiple scarb versions
RUN git clone --depth 1 https://github.com/asdf-vm/asdf.git "$HOME/.asdf" && \
    echo ". $HOME/.asdf/asdf.sh" >> "$HOME/.bashrc" && \
    echo ". $HOME/.asdf/asdf.sh" >> "$HOME/.profile"

SHELL ["/bin/bash", "-c"]
RUN source "$HOME/.asdf/asdf.sh" && asdf plugin add scarb && asdf install scarb 0.7.0  && asdf install scarb 2.6.5

RUN --mount=type=cache,target=/root/.cache \
    source "$HOME/.asdf/asdf.sh" \
    && chmod +x ./scripts/extract_abi.sh \
    && git submodule update --init --recursive \
	&& cp .env.example .env \
    && cd lib/kakarot && uv sync --all-extras --dev && make build \
    && mv build/ssj/contracts_Cairo1Helpers.contract_class.json build/cairo1_helpers.json && rm -fr build/ssj && cd ../.. \
    && ./scripts/extract_abi.sh

COPY . .

### Build the RPC server
# Define ARG for build platform
FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder

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

# Builds dependencies
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --features hive --recipe-path recipe.json

# Build application
COPY . .
RUN --mount=type=cache,target=/root/.cache/cargo cargo build \
    --features hive --release --target-dir /usr/src/rpc/target && \
    cargo build \
    --bin hive_genesis --release --features "testing,binaries" --target-dir /usr/src/rpc/target && \
    cargo build \
    --bin hive_chain --release --features "testing,binaries" --target-dir /usr/src/rpc/target

FROM ubuntu:24.10 as base
# Install any necessary dependencies
RUN apt-get update && apt-get install --no-install-recommends -y tini curl jq libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*

# Environment
#############
### Indexer environment variables
#### Indexer environment variables
ENV ALLOW_ENV_FROM_ENV=DEBUG,APIBARA_AUTH_TOKEN,STARTING_BLOCK,STREAM_URL,SINK_TYPE,MONGO_CONNECTION_STRING,MONGO_DATABASE_NAME,STARKNET_NETWORK,KAKAROT_ADDRESS,ALLOW_NET,MAX_FELTS_IN_CALLDATA,DEFAULT_BLOCK_GAS_LIMIT
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
ENV RUST_LOG=info
ENV MAX_FELTS_IN_CALLDATA=30000
ENV MAX_LOGS=10000
ENV DEFAULT_BLOCK_GAS_LIMIT=7000000
ENV RELAYER_PRIVATE_KEY=0x2bbf4f9fd0bbb2e60b0316c1fe0b76cf7a4d0198bd493ced9b8df2a3a24d68a
ENV RELAYERS_ADDRESSES=0xb3ff441a68610b30fd5e2abbf3a1548eb6ba6f3559f2862bf2dc757e5828ca
ENV RUST_LOG=info

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
