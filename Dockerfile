FROM rust:1.92-slim AS builder
WORKDIR /usr/src/build-hook
COPY . .
RUN cargo install --path .

FROM debian:trixie-slim
COPY --from=builder /usr/local/cargo/bin/build-hook /usr/local/bin/build-hook
WORKDIR /app
CMD ["build-hook"]

