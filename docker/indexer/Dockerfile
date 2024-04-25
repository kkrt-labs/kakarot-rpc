FROM quay.io/apibara/sink-mongo:0.7.0 as indexer

FROM debian:bookworm as apibara-build
RUN apt-get update && apt-get install -y patchelf && rm -rf /var/lib/apt/lists/*

ARG BUILDPLATFORM
ARG APIBARA_SINK_BIN_DIR

COPY --from=indexer /nix/store/${APIBARA_SINK_BIN_DIR}-apibara-sink-mongo-0.7.0/bin/apibara-sink-mongo /usr/local/bin/sink-mongo

RUN case $BUILDPLATFORM in \
    "linux/amd64") \
    patchelf --set-interpreter /lib64/ld-linux-x86-64.so.2 /usr/local/bin/sink-mongo; \
    ;; \
    "linux/arm64") \
    patchelf --set-interpreter /lib/ld-linux-aarch64.so.1 /usr/local/bin/sink-mongo; \
    ;; \
    *) \
    echo "Unknown BUILDPLATFORM: $BUILDPLATFORM"; \
    exit 1; \
    ;; \
    esac


FROM debian:latest as base

WORKDIR /code

COPY ./indexer .


FROM debian:latest

COPY --from=base /code /usr/src/app/code
COPY --from=apibara-build /usr/local/bin/sink-mongo /usr/local/bin/sink-mongo

CMD ["sink-mongo", "run", "/usr/src/app/code/src/main.ts"]
