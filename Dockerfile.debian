####################################################################################################
## Builder
####################################################################################################
FROM rust:latest AS builder

RUN update-ca-certificates

# Create appuser
ENV USER=freighter
ENV UID=10001

RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    "${USER}"


WORKDIR /freighter

COPY ./ .

# We no longer need to use the x86_64-unknown-linux-musl target
RUN cargo build --release

####################################################################################################
## Final image
####################################################################################################
FROM debian:bullseye-slim

USER root

RUN apt update && apt install -y ca-certificates

# Import from builder.
COPY --from=builder /etc/passwd /etc/passwd
COPY --from=builder /etc/group /etc/group

RUN mkdir -p /freighter/data

RUN chown freighter /freighter/data

WORKDIR /freighter

# Copy our build
COPY --from=builder /freighter/target/release/freighter /usr/local/bin

# Use an unprivileged user.
USER freighter:freighter

#CMD ["/freighter/freighter"]
