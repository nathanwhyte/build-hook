FROM rust:1.92-slim AS builder

# compile Rust binary
WORKDIR /usr/src/build-hook
COPY . .
RUN cargo install --path .

# install curl
RUN apt-get update && apt-get install -y curl

# install kubectl
WORKDIR /usr/src/kubectl
RUN curl -LO "https://dl.k8s.io/release/v1.34.3/bin/linux/amd64/kubectl"
RUN install -o root -g root -m 0755 kubectl /usr/local/bin/kubectl

FROM debian:trixie-slim
COPY --from=builder /usr/local/cargo/bin/build-hook /usr/local/bin/build-hook
COPY --from=builder /usr/local/bin/kubectl /usr/local/bin/kubectl
WORKDIR /app
CMD ["build-hook"]

