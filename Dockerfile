FROM debian:trixie-slim AS binaries
# install curl
RUN apt update && apt install -y curl

# install kubectl
WORKDIR /usr/src/kubectl
RUN curl -LO "https://dl.k8s.io/release/v1.34.3/bin/linux/amd64/kubectl"
RUN install -o root -g root -m 0755 kubectl /usr/local/bin/kubectl

FROM rust:1.92-slim AS builder

# Install OpenSSL development libraries and pkg-config
RUN apt-get update && apt-get install -y libssl-dev pkg-config && rm -rf /var/lib/apt/lists/*

# compile Rust binary
WORKDIR /usr/src/build-hook
COPY . .
RUN cargo install --path .

FROM debian:trixie-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=binaries /usr/local/bin/kubectl /usr/local/bin/kubectl
COPY --from=builder /usr/local/cargo/bin/build-hook /usr/local/bin/build-hook
WORKDIR /app
CMD ["build-hook"]

