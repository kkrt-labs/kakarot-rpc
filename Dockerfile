FROM rust:1.64 as builder

# Set working directory
WORKDIR /usr/src/rpc

# Copy source code
COPY . .

RUN rustup self update

# Build the application
RUN cargo build --all --release

# Create a new container from scratch to reduce image size
FROM debian:buster-slim

# Install any necessary dependencies
RUN apt-get update && apt-get install -y libssl-dev ca-certificates tini && rm -rf /var/lib/apt/lists/*

# Set the working directory
WORKDIR /usr/src/app

# Copy the built binary from the previous container
COPY --from=builder /usr/src/rpc/target/release/kakarot-rpc /usr/local/bin

# Expose the port that the RPC server will run on
EXPOSE 9545
EXPOSE 3030

# this is required to have exposing ports work from docker, the default is not this.
ENV KAKAROT_HTTP_RPC_ADDRESS="0.0.0.0:9545"

# Seen in https://github.com/eqlabs/pathfinder/blob/4ab915a830953ed6f02af907937b46cb447d9a92/Dockerfile#L120 - 
# Allows for passing args down to the underlying binary easily
ENTRYPOINT ["/usr/bin/tini", "--", "/usr/local/bin/kakarot-rpc"]

# empty CMD is needed and cannot be --help because otherwise configuring from
# environment variables only would be impossible and require a workaround.
CMD []

