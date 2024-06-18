FROM debian:latest as apibara-build

RUN apt-get update && apt-get install --yes jq curl gzip

ARG APIBARA_DOWNLOAD_URL

# Install apibara cli binary
RUN curl -sL $APIBARA_DOWNLOAD_URL | gzip -d > sink-mongo

FROM debian:latest

COPY ./indexer /usr/src/app/code
COPY --from=apibara-build sink-mongo /usr/local/bin/sink-mongo
RUN chmod +x /usr/local/bin/sink-mongo

CMD ["sink-mongo", "run", "/usr/src/app/code/src/main.ts"]
