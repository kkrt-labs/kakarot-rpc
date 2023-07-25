# Start from a small, trusted base image
FROM alpine:3.14

# Install jq
RUN apk add --no-cache jq

# Set a working directory
WORKDIR /app

# Copy the script to the container
COPY parse.sh .

# Make the script executable
RUN chmod +x parse.sh

# Run the script
CMD ["./parse.sh"]
