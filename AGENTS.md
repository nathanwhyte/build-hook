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
- `config.toml`: runtime configuration
