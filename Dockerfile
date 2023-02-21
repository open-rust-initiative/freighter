ARG RUNTIME_IMAGE
FROM $RUNTIME_IMAGE
COPY target/release/freighter /usr/local/bin
ENTRYPOINT freighter
