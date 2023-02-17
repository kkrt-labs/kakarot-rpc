FROM rust:1.66 as builder

# Set working directory
WORKDIR /usr/src/rpc

# Copy source code
COPY . .

# Build the application
RUN cargo build --all --release

# Create a new container from scratch to reduce image size
FROM debian:buster-slim

# Install any necessary dependencies
RUN apt-get update && apt-get install -y libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*

# RUN update-ca-certificates

# Set the working directory
WORKDIR /usr/src/app

# Copy the built binary from the previous container
COPY --from=builder /usr/src/rpc/target/release/kakarot-rpc .

# Expose the port that the RPC server will run on
EXPOSE 3030

# Run the binary
CMD ["./kakarot-rpc"]

