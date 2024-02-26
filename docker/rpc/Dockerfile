FROM python:3.9.13 as compiler
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

COPY .git ./.git
COPY .gitmodules .gitmodules
COPY Makefile Makefile
COPY scripts scripts
RUN make setup

# Define ARG for build platform
FROM --platform=$BUILDPLATFORM rust:1.75 as builder

# Set ARG for build and target platform
ARG BUILDPLATFORM
ARG TARGETPLATFORM

# Set working directory
WORKDIR /usr/src/rpc

COPY --from=compiler /usr/src/compiler/artifacts /usr/src/rpc/artifacts

# Copy source code
COPY . .

# Cross-compile the application for a given platform
RUN build_platform() { \
    ARCH=$1; \
    COMPILER=$2; \
    LINKER=$3; \
    echo "Building for $TARGETPLATFORM on $BUILDPLATFORM"; \
    # Add the specified Rust target architecture
    rustup target add $ARCH; \
    # Update package lists and install the specified compiler
    apt-get update && apt-get -y install $COMPILER libclang-dev; \
    # Determine the BINDGEN_EXTRA_CLANG_ARGS based on comparison between BUILDPLATFORM and TARGETPLATFORM
    if [ "$BUILDPLATFORM" = "$TARGETPLATFORM" ]; then \
    BINDGEN_EXTRA_CLANG_ARGS=""; \
    else \
    BINDGEN_EXTRA_CLANG_ARGS="--sysroot /usr/${LINKER%-gcc}"; \
    fi; \
    echo "Using BINDGEN_EXTRA_CLANG_ARGS: $BINDGEN_EXTRA_CLANG_ARGS"; \
    # Build the Rust application for the specified target
    BINDGEN_EXTRA_CLANG_ARGS=$BINDGEN_EXTRA_CLANG_ARGS cargo build --all --release \
    --target=$ARCH; \
    # Copy the built binary to a common release directory
    cp /usr/src/rpc/target/$ARCH/release/kakarot-rpc /usr/src/rpc/target/release/; \
    # Build the Rust application for the specified target for hive
    BINDGEN_EXTRA_CLANG_ARGS=$BINDGEN_EXTRA_CLANG_ARGS cargo build --all --release \
    --features hive --target=$ARCH; \
    # Copy the built binary to a common release directory
    cp /usr/src/rpc/target/$ARCH/release/kakarot-rpc /usr/src/rpc/target/release/kakarot-rpc-hive; \
    # Build the hive genesis binary
    BINDGEN_EXTRA_CLANG_ARGS=$BINDGEN_EXTRA_CLANG_ARGS cargo build --all --release \
    --bin hive_genesis --features testing --target=$ARCH; \
    # Copy the genesis binary to a common release directory
    cp /usr/src/rpc/target/$ARCH/release/hive_genesis /usr/src/rpc/target/release/; \
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
FROM debian:bookworm

# Install any necessary dependencies
RUN apt-get update && apt-get install -y libssl-dev ca-certificates tini curl && rm -rf /var/lib/apt/lists/*

# Set the working directory
WORKDIR /usr/src/app

# Copy the built binary from the previous container
COPY --from=builder /usr/src/rpc/target/release/kakarot-rpc /usr/local/bin
COPY --from=builder /usr/src/rpc/target/release/kakarot-rpc-hive /usr/local/bin
COPY --from=builder /usr/src/rpc/target/release/hive_genesis /usr/local/bin

# Expose the port that the RPC server will run on
EXPOSE 9545
EXPOSE 3030

# this is required to have exposing ports work from docker, the default is not this.
ENV KAKAROT_RPC_URL="0.0.0.0:3030"

# Add a health check to make sure the service is healthy
HEALTHCHECK --interval=3s --timeout=5s --start-period=1s --retries=5 \
    CMD curl --request POST \
    --header "Content-Type: application/json" \
    --data '{"jsonrpc": "2.0", "method": "eth_chainId", "id": 1}' http://${KAKAROT_RPC_URL} || exit 1

# Seen in https://github.com/eqlabs/pathfinder/blob/4ab915a830953ed6f02af907937b46cb447d9a92/Dockerfile#L120 - 
# Allows for passing args down to the underlying binary easily
ENTRYPOINT ["/usr/bin/tini", "--", "/usr/local/bin/kakarot-rpc"]

# empty CMD is needed and cannot be --help because otherwise configuring from
# environment variables only would be impossible and require a workaround.
CMD []
