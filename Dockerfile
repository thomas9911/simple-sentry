# Using the `rust-musl-builder` as base image, instead of 
# the official Rust toolchain
FROM clux/muslrust:1.88.0-stable AS chef
USER root
RUN cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
ARG TARGETPLATFORM
RUN case "$TARGETPLATFORM" in \
    "linux/amd64") echo "x86_64-unknown-linux-musl" > /tmp/rust-target ;; \
    "linux/arm64") curl -LO https://github.com/thomas9911/simple-sentry/releases/download/v0.1.0/aarch64-linux-musl-cross.tgz && \
    tar -xzf aarch64-linux-musl-cross.tgz && mv aarch64-linux-musl-cross/bin/* /usr/local/bin/ && \
    echo "aarch64-unknown-linux-musl" > /tmp/rust-target ;; \
    *) echo "Unsupported platform: $TARGETPLATFORM" && exit 1 ;; \
    esac
COPY --from=planner /app/recipe.json recipe.json
# Notice that we are specifying the --target flag!
RUN cargo chef cook --release --target $(cat /tmp/rust-target) --recipe-path recipe.json
COPY . .
RUN cargo build --release --target $(cat /tmp/rust-target) --bin simple-sentry

FROM alpine:3.19 AS runtime
RUN addgroup -S myuser && adduser -S myuser -G myuser
COPY --from=builder /app/target/*/release/simple-sentry /usr/local/bin/
USER myuser
ENV SIMPLE_SENTRY_DB=/home/myuser/simple-sentry/data.db
RUN mkdir -p /home/myuser/simple-sentry
CMD ["/usr/local/bin/simple-sentry"]
