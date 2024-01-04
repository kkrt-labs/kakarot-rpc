# trunk-ignore-all(terrascan/AC_DOCKER_0047)
FROM ghcr.io/keep-starknet-strange/madara:v0.6.0 as madara


# trunk-ignore(terrascan/AC_DOCKER_0041)
# trunk-ignore(hadolint/DL3007)
# trunk-ignore(trivy/DS001)
FROM ghcr.io/kkrt-labs/kakarot-rpc/node:latest

RUN groupadd -r appgroup && useradd -r -g appgroup appuser

USER appuser

# Install svn to download Madara genesis config files
# trunk-ignore(hadolint/DL3008)
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
# Test running on this port https://github.com/ethereum/hive/blob/master/simulators/ethereum/rpc/helper.go#L50
ENV KAKAROT_HTTP_RPC_ADDRESS="0.0.0.0:8545"
ENV STARKNET_NETWORK=http://0.0.0.0:9944
ENV RUST_LOG=info
# https://github.com/kkrt-labs/kakarot-rpc/blob/main/crates/hive-utils/src/hive/genesis.rs#L59
ENV KAKAROT_ADDRESS="0x9001"
ENV DEPLOYER_ACCOUNT_ADDRESS="0x9003"
# https://github.com/kkrt-labs/kakarot-rpc/blob/main/crates/core/src/test_utils/constants.rs#L14
ENV DEPLOYER_ACCOUNT_PRIVATE_KEY="0x0288a51c164874bb6a1ca7bd1cb71823c234a86d0f7b150d70fa8f06de645396"

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
EXPOSE 8545

# Misc
RUN curl https://get.starkli.sh | sh
ENV PATH="${PATH}:/root/.starkli/bin"
RUN starkliup

# Copy start script
COPY crates/hive-utils/scripts/start.sh /start.sh
RUN chmod +x /start.sh

ENTRYPOINT ["/usr/bin/tini", "--", "/start.sh"]

CMD []
