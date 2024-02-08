# trunk-ignore-all(terrascan/AC_DOCKER_0047)

# CairoVM Chain
FROM greged93/katana:v0.4.4 as katana

# Indexer service
### Apibara DNA indexer
FROM quay.io/apibara/starknet:1.2.0 as apibara
### Indexer transform plugin
#### First, clone the indexer repository
FROM docker.io/alpine/git:latest as indexer-cloner
WORKDIR /code
RUN git clone -v https://github.com/kkrt-labs/kakarot-indexer.git"
#### Then, get the indexer binary
FROM quay.io/apibara/sink-mongo as indexer
#### MongoDB
FROM mongo:6.0.8 as mongo

# Ethereum RPC Server
FROM ghcr.io/kkrt-labs/kakarot-rpc/node:v0.2.1
WORKDIR /usr/src/app


# Environment
#############
### Indexer environment variables
#### MongoDB environment variables
ENV MONGO_INITDB_ROOT_USERNAME: mongo
ENV MONGO_INITDB_ROOT_PASSWORD: mongo
#### Indexer environment variables
ENV ALLOW_ENV_FROM_ENV=DEBUG,APIBARA_AUTH_TOKEN,STARTING_BLOCK,STREAM_URL,SINK_TYPE,MONGO_CONNECTION_STRING,MONGO_DATABASE_NAME
ENV DEBUG=""
ENV APIBARA_AUTH_TOKEN=""
ENV MONGO_CONNECTION_STRING=mongodb://mongo:mongo@mongo:27017
ENV MONGO_DATABASE_NAME=kakarot-local
ENV STARTING_BLOCK=0
ENV STREAM_URL=http://apibara-dna:7171
ENV SINK_TYPE=mongo
### Kakarot RPC environment variables
### Port 8545: https://github.com/ethereum/hive/blob/master/simulators/ethereum/rpc/helper.go#L50
ENV KAKAROT_RPC_URL=0.0.0.0:8545
ENV STARKNET_NETWORK=http://localhost:5050
ENV RUST_LOG=kakarot_rpc=info
ENV MONGO_CONNECTION_STRING=mongodb://mongo:mongo@mongo:27017
# ENV MONGO_DATABASE_NAME=kakarot-local - already set in indexer

# Add a health check to make sure the service is healthy
HEALTHCHECK --interval=3s --timeout=5s --start-period=1s --retries=5 \
    CMD curl --request POST \
    --header "Content-Type: application/json" \
    --data '{"jsonrpc": "2.0", "method": "eth_chainId", "id": 1}' http://${KAKAROT_RPC_URL} || exit 1

# Ports
#######
# 9615 Prometheus exporter
# 30333 P2P communication
# 8545 Ethereum RPC
# 7171 DNA indexer
# 27017 MongoDB
EXPOSE 5050 9615 30333 8545 7171 27017

# Kakarot RPC needs the following variables to function
# "KAKAROT_ADDRESS=
# "DEPLOYER_ACCOUNT_ADDRESS=
# "PROXY_ACCOUNT_CLASS_HASH=
# "EXTERNALLY_OWNED_ACCOUNT_CLASS_HASH=
# "CONTRACT_ACCOUNT_CLASS_HASH=
# Make sure they are set in the environment after Katana has created a genesis file.

# Copy binaries and dependencies
################################
### CairoVM chain service
COPY --from=katana /usr/local/bin/katana /usr/local/bin/katana
### Indexer service
#### We need the DNA indexer binary
COPY --from=apibara /usr/local/bin/starknet /usr/local/bin/starknet
#### We need the indexer typescript code and the binary that knows how to run it
COPY --from=indexer-cloner /code /code
COPY --from=indexer /usr/local/bin/sink-mongo /usr/local/bin/sink-mongo
#### We need the mongo binary
COPY --from=mongo /bin/mongod /usr/local/bin/mongod


# Copy start script
COPY docker/hive/start.sh /start.sh
RUN chmod +x /start.sh

ENTRYPOINT ["/usr/bin/tini", "--", "/start.sh"]

CMD []
