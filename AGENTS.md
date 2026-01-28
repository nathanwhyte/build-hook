# Codex agent guide for build-hook

This repository is a small Rust service that exposes a build hook API using Axum.
Use this file as the local instructions for agents working in this repo.

## Project overview
- Language: Rust (edition 2024)
- Entrypoint: `src/main.rs`
- Web API: `src/api.rs` (Axum router, healthcheck, build hook handler)
- Auth: `src/auth.rs` (Bearer token auth via `BEARER_TOKENS` env)
- Config: `config.toml` parsed by `src/config.rs`

## Development workflow
- Build: `cargo build`
- Run: `cargo run`
- Test: `cargo test`
- Lint/format (if added later): prefer `cargo fmt` and `cargo clippy`

## Codebase conventions
- Prefer small, focused functions; keep handlers thin.
- Use `tracing` for logging; avoid `println!`.
- Keep configuration validation in `src/config.rs`.
- Favor `Result` returns over panics when adding new fallible logic.

## Runtime/config notes
- `config.toml` is required at startup.
- Auth expects `Authorization: Bearer <token>` where tokens come from
  the comma-separated `BEARER_TOKENS` environment variable.
- The service listens on `0.0.0.0:3000`.

## When editing
- Update or add tests where behavior changes are non-trivial.
- Keep changes ASCII-only unless the file already uses Unicode.
- Avoid sweeping refactors unless requested; make the smallest change.

## Files to know
- `src/main.rs`: bootstraps tracing and starts the API server
- `src/api.rs`: routing, healthcheck, build-hook handler
- `src/auth.rs`: auth middleware and token parsing
- `src/config.rs`: config schema, validation, and logging
- `src/buildx.rs`: Docker Buildx builder initialization and management
- `src/project/image.rs`: image building logic using buildx
- `config.toml`: runtime configuration

## BuildKit architecture

The service uses Docker Buildx with the **remote driver** to connect to a BuildKit daemon
deployed separately in Kubernetes. This architecture avoids cgroup v2 exec issues that occur
with the `kubernetes` driver on systems using containerd with cgroup v2.

### Components
- `buildkitd` Deployment: Runs the BuildKit daemon with TCP listener on port 1234
- `buildkitd` Service: Exposes buildkitd at `tcp://buildkitd.build.svc.cluster.local:1234`
- `hook` Deployment: The build-hook API that uses the remote driver to connect to buildkitd

### Why remote driver?
The Kubernetes driver for Buildx uses `kubectl exec` to communicate with BuildKit pods.
On clusters with cgroup v2 and containerd, exec operations into privileged containers
fail with cgroup path errors. The remote driver uses TCP instead, avoiding the exec issue.
