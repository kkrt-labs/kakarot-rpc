# Use cargo-chef to handle Rust dependencies
FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

# Prepare the recipe for caching dependencies
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Install build dependencies and build the application
FROM chef AS builder 
COPY --from=planner /app/recipe.json recipe.json

# Install necessary system dependencies once
RUN apt-get update && \
    apt-get -y upgrade && \
    apt-get install --no-install-recommends -y libclang-dev libssl-dev ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# Build dependencies for caching
RUN cargo chef cook --release --recipe-path recipe.json

# Build the application using cached dependencies
COPY . .
RUN --mount=type=cache,target=/root/.cache cargo build --release --bin kakarot-rpc

# Runtime stage with minimal dependencies
FROM debian:bookworm-slim AS runtime
WORKDIR /usr/src/app

# Install runtime dependencies and clean up
RUN apt-get update && \
    apt-get -y upgrade && \
    apt-get install --no-install-recommends -y tini curl && \
    rm -rf /var/lib/apt/lists/*

# Copy only the necessary binaries
COPY --from=builder /app/target/release/kakarot-rpc /usr/local/bin/kakarot-rpc
COPY ./indexer /usr/src/indexer

# Expose necessary ports
EXPOSE 9545 3030 9615

# Configure environment variables
ENV KAKAROT_RPC_URL="0.0.0.0:3030"

# Add a health check for the RPC server
HEALTHCHECK --interval=3s --timeout=5s --start-period=1s --retries=5 \
    CMD curl --request POST \
    --header "Content-Type: application/json" \
    --data '{"jsonrpc": "2.0", "method": "eth_chainId", "id": 1}' http://${KAKAROT_RPC_URL} || exit 1

# Set up the entry point using tini
ENTRYPOINT ["/usr/bin/tini", "--", "/usr/local/bin/kakarot-rpc"]

# Empty CMD to allow configuration via environment variables
CMD []
