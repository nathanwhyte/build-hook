# build-hook

Small Rust service that exposes a build hook API over HTTP using [axum](https://docs.rs/axum/latest/axum/index.html).
It authenticates requests with bearer tokens and builds container images
through Docker `buildx` using the remote driver connected to an
external [BuildKit](https://docs.docker.com/build/buildkit/configure/) daemon.

- Fills the role of GitHub actions on pushes to main, but for free.
- Triggers image builds and resource deployments for configured projects on request.

## Requirements

- Rust `stable` toolchain (edition 2024)
- Docker with Buildx configured to reach a BuildKit daemon

## Configuration

- `config.toml` is required at startup and holds runtime settings.
- `BEARER_TOKENS` is a comma-separated list of valid bearer tokens.

## Run locally

```bash
cargo build
cargo run
```

The service listens on `0.0.0.0:3000`.
