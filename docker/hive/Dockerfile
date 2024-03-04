# trunk-ignore-all(terrascan/AC_DOCKER_0047)

# CairoVM Chain
FROM ghcr.io/dojoengine/dojo:v0.6.0-alpha.2 as katana

# Indexer service
### Apibara DNA indexer and indexer
FROM quay.io/apibara/starknet:1.4.0 as apibara
FROM quay.io/apibara/sink-mongo:0.5.3 as indexer
FROM debian:bookworm as apibara-build
RUN apt-get update && apt-get install -y patchelf && rm -rf /var/lib/apt/lists/*
# Run `docker image inspect apibara/starknet:1.4.0` to get the exact path
COPY --from=apibara /nix/store/nkskc1iii2s658r2mx8bsgk11p6ibz82-apibara-starknet-1.4.0/bin/apibara-starknet /usr/local/bin/starknet
COPY --from=indexer /nix/store/3iqnrcirqpg4s7zdy1wdh0dq17jwzmlc-apibara-sink-mongo-0.5.3/bin/apibara-sink-mongo /usr/local/bin/sink-mongo
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

### Indexer transform plugin
#### First, clone the indexer repository
FROM docker.io/alpine/git:latest as indexer-cloner
WORKDIR /code
RUN git clone -v "https://github.com/kkrt-labs/kakarot-indexer.git"

#### MongoDB
FROM mongo:6.0.8 as mongo

# Ethereum RPC Server
FROM ghcr.io/kkrt-labs/kakarot-rpc/node:v0.2.4 as rpc

FROM debian:bookworm-slim as base
# Install any necessary dependencies
RUN apt-get update && apt-get install -y tini curl jq && rm -rf /var/lib/apt/lists/*

# Environment
#############
### Indexer environment variables
#### Indexer environment variables
ENV ALLOW_ENV_FROM_ENV=DEBUG,APIBARA_AUTH_TOKEN,STARTING_BLOCK,STREAM_URL,SINK_TYPE,MONGO_CONNECTION_STRING,MONGO_DATABASE_NAME
ENV DEBUG=""
ENV APIBARA_AUTH_TOKEN=""
ENV MONGO_CONNECTION_STRING=mongodb://localhost:27017
ENV MONGO_DATABASE_NAME=kakarot-local
ENV STARTING_BLOCK=0
ENV STREAM_URL=http://localhost:7171
ENV SINK_TYPE=mongo
ENV KATANA_ACCOUNT_ADDRESS=0xb3ff441a68610b30fd5e2abbf3a1548eb6ba6f3559f2862bf2dc757e5828ca
ENV KATANA_PRIVATE_KEY=0x2bbf4f9fd0bbb2e60b0316c1fe0b76cf7a4d0198bd493ced9b8df2a3a24d68a
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
COPY --from=indexer-cloner /code /usr/src/app/code
COPY --from=apibara-build /usr/local/bin/sink-mongo /usr/local/bin/sink-mongo

#### We need the mongo binary
COPY --from=mongo /bin/mongod /usr/local/bin
RUN mkdir -p /data/db


# Generate the genesis
COPY --from=rpc /usr/local/bin/kakarot-rpc-hive /usr/local/bin/kakarot-rpc
COPY --from=rpc /usr/local/bin/hive_genesis /usr/local/bin
COPY --from=rpc /usr/local/bin/hive_chain /usr/local/bin
RUN mkdir -p /genesis/contracts
COPY ./lib/kakarot/build /genesis/contracts

# Copy start script
COPY docker/hive/start.sh /start.sh
RUN chmod +x /start.sh

ENTRYPOINT ["/usr/bin/tini", "--", "/start.sh"]

CMD []
