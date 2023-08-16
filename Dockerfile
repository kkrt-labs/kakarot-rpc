# Define ARG for build platform
FROM --platform=$BUILDPLATFORM rust:1.64 as builder

# Set ARG for target platform
ARG TARGETPLATFORM

# Set working directory
WORKDIR /usr/src/rpc

# Copy source code
COPY . .

# Cross-compile the application for a given platform
RUN build_platform() { \
        ARCH=$1; \
        COMPILER=$2; \
        LINKER=$3; \
        echo "Building for $TARGETPLATFORM"; \
         # Add the specified Rust target architecture
        rustup target add $ARCH; \
        # Update package lists and install the specified compiler
        apt-get update && apt-get -y install $COMPILER; \
        # Build the Rust application for the specified target
        cargo build --all --release \
          --target=$ARCH \
          --config target.$ARCH.linker=\"$LINKER\"; \
        # Copy the built binary to a common release directory
        cp /usr/src/rpc/target/$ARCH/release/kakarot-rpc /usr/src/rpc/target/release/; \
    } \
    && rustup self update \
    && case "$TARGETPLATFORM" in \
        "linux/amd64") \
            build_platform "x86_64-unknown-linux-gnu" "gcc-x86-64-linux-gnu" "x86_64-linux-gnu-gcc"; \
            ;; \
        "linux/arm64") \
            build_platform "aarch64-unknown-linux-gnu" "gcc-aarch64-linux-gnu" "aarch64-linux-gnu-gcc"; \
            ;; \
        *) \
            echo "Unknown TARGETPLATFORM: $TARGETPLATFORM"; \
            exit 1; \
            ;; \
    esac

# Create a new container from scratch to reduce image size
FROM debian:bullseye

# Install any necessary dependencies
RUN apt-get update && apt-get install -y libssl-dev ca-certificates tini curl && rm -rf /var/lib/apt/lists/*

# Set the working directory
WORKDIR /usr/src/app

# Copy the built binary from the previous container
COPY --from=builder /usr/src/rpc/target/release/kakarot-rpc /usr/local/bin

# Expose the port that the RPC server will run on
EXPOSE 9545
EXPOSE 3030

# this is required to have exposing ports work from docker, the default is not this.
ENV KAKAROT_HTTP_RPC_ADDRESS="0.0.0.0:9545"

# Add a health check to make sure the service is healthy
HEALTHCHECK --interval=3s --timeout=5s --start-period=1s --retries=5 \
  CMD curl --request POST \
    --header "Content-Type: application/json" \
    --data '{"jsonrpc": "2.0", "method": "eth_chainId", "id": 1}' http://${KAKAROT_HTTP_RPC_ADDRESS} || exit 1

# Seen in https://github.com/eqlabs/pathfinder/blob/4ab915a830953ed6f02af907937b46cb447d9a92/Dockerfile#L120 - 
# Allows for passing args down to the underlying binary easily
ENTRYPOINT ["/usr/bin/tini", "--", "/usr/local/bin/kakarot-rpc"]

# empty CMD is needed and cannot be --help because otherwise configuring from
# environment variables only would be impossible and require a workaround.
CMD []
