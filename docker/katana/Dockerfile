FROM ghcr.io/dojoengine/dojo:latest as dojo

RUN apt-get update && apt-get install -y curl

HEALTHCHECK --interval=10s --timeout=15s --start-period=1s --retries=5 \
  CMD curl --request POST \
    --header "Content-Type: application/json" \
    --data '{"jsonrpc": "2.0", "method": "starknet_chainId", "id": 1}' http://0.0.0.0:5050 || exit 1
