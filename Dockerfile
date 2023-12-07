# stage 1 - generate recipe file for dependencies
FROM rust:1.74.0-alpine3.18 as planner

WORKDIR /app
COPY ./Cargo.toml ./Cargo.toml
COPY ./Cargo.lock ./Cargo.lock

# stage 2 - Build our project
FROM rust:1.74.0-alpine3.18 as builder

ARG channel=stable
## Build our nanocl daemon binary
COPY --from=planner /app /app
WORKDIR /app
RUN apk add --update alpine-sdk musl-dev g++ make libpq-dev openssl-dev git upx perl build-base
COPY ./src ./src
COPY .git ./.git
ENV CHANNEL=$channel
ENV RUSTFLAGS="-C target-feature=+crt-static"
RUN RUSTFLAGS="-C target-feature=-crt-static" cargo build --release
RUN upx --lzma --best /app/target/release/docker-events-apprise \
  && cp /app/target/release/docker-events-apprise /bin/docker-events-apprise

# stage 4 - create runtime image
FROM scratch

LABEL org.opencontainers.image.source https://github.com/anonkey/docker-events-apprise
LABEL org.opencontainers.image.description Docker events apprise notifier

COPY --from=builder /bin/docker-events-apprise /bin/docker-events-apprise

ENTRYPOINT ["docker-events-apprise", "/config.yml", "http://localhost:8000"]
