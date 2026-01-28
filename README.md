# build-hook

Small Rust service that exposes a build hook API over HTTP using [axum](https://docs.rs/axum/latest/axum/index.html).
It authenticates requests with bearer tokens and builds container images
through Docker `buildx` using the remote driver connected to an
external [BuildKit](https://docs.docker.com/build/buildkit/configure/) daemon.

- Fills the role of GitHub actions on pushes to main, but for free.
- Triggers image builds and resource deployments for configured projects on request.

## Requirements

- Rust `stable` toolchain (edition 2024)
- Docker with `buildx` configured to reach a BuildKit daemon

## Configuration

- `config.toml` is required at startup and holds runtime settings.
- `BEARER_TOKENS` is a comma-separated list of valid bearer tokens _(no particular format)_.

### config.toml format

```toml
[app]
# Compatible with any OCI registry, e.g. ghcr.io, docker.io, etc.
registry = "registry.example.com"

[[projects]]
name = "My Service"
# `slug` is the unique identifier used to load the correct config
slug = "my-service"

[projects.code]
url = "https://github.com/org/repo"
branch = "main"

[[projects.image]]
repository = "my-service/api"
location = "Dockerfile"
tag = "release"

# You can define multiple images per project
[[projects.image]]
repository = "my-service/worker"
location = "Dockerfile"
tag = "release"

[projects.deployments]
namespace = "my-service"
resources = ["deployment/api"]
```

### Configuration fields

- `app.registry`: Base registry used to tag images.
- `projects`: List of buildable projects.
- `projects.name`: Display name for the project.
- `projects.slug`: Unique slug used for routing and cloning.
- `projects.code.url`: HTTPS Git repository URL.
- `projects.code.branch`: Branch to build from.
- `projects.image`: One or more images to build per project.
- `projects.image.repository`: Repository path under the registry.
- `projects.image.location`: Dockerfile path relative to repo root.
- `projects.image.tag`: Tag to apply to the image.
- `projects.deployments.namespace`: Kubernetes namespace for rollout restarts.
- `projects.deployments.resources`: Kubernetes resources to restart.
