# TODO: Use official madara image when https://github.com/keep-starknet-strange/madara/issues/982 is closed
# FROM ghcr.io/keep-starknet-strange/madara:v0.1.0.experimental.3 as madara
FROM fredtupas/madara as madara

FROM ghcr.io/kkrt-labs/kakarot-rpc/node:latest

# Install svn to download Madara genesis config files
RUN apt-get -y update; \
    apt-get install -y --no-install-recommends subversion; \
    apt-get autoremove -y; \
    apt-get clean; \
    rm -rf /var/lib/apt/lists/*

# Copy Madara binary
COPY --from=madara /madara-bin /madara-bin

# Madara Path
ENV MADARA_PATH=/root/.madara
ENV MADARA_CONFIG_PATH=${MADARA_PATH}/configs
ENV KAKAROT_CONTRACTS_PATH=${MADARA_PATH}/cairo-contracts/kakarot

# Download Madara genesis config to source path
RUN svn export https://github.com/keep-starknet-strange/madara/trunk/configs ${MADARA_CONFIG_PATH}

# Copy combined genesis.json to source path
COPY .hive/genesis.json ${MADARA_CONFIG_PATH}/genesis-assets/genesis.json

# Copy compiled Kakarot contracts to destination path of compiled contracts
COPY ./lib/kakarot/build ${KAKAROT_CONTRACTS_PATH}

# RPC environment variables
ENV KAKAROT_HTTP_RPC_ADDRESS="0.0.0.0:3030"
ENV STARKNET_NETWORK=http://0.0.0.0:9944
ENV RUST_LOG=trace
# https://github.com/kkrt-labs/kakarot-rpc/blob/main/crates/hive-utils/src/hive/genesis.rs#L59
ENV KAKAROT_ADDRESS="0x9001"

# Add a health check to make sure the service is healthy
HEALTHCHECK --interval=3s --timeout=5s --start-period=1s --retries=5 \
  CMD curl --request POST \
    --header "Content-Type: application/json" \
    --data '{"jsonrpc": "2.0", "method": "eth_chainId", "id": 1}' http://${KAKAROT_HTTP_RPC_ADDRESS} || exit 1

# 9444 JSON-RPC server
# 9615 Prometheus exporter
# 30333 P2P communication
EXPOSE 9944 9615 30333

# Expose the port that the RPC server will run on
EXPOSE 9545
EXPOSE 3030

# Copy start script
COPY crates/hive-utils/scripts/start.sh /start.sh
RUN chmod +x /start.sh

# Misc
RUN curl https://get.starkli.sh | sh
ENV PATH="${PATH}:/root/.starkli/bin"
RUN starkliup

ENTRYPOINT ["/usr/bin/tini", "--", "/start.sh"]

CMD []
