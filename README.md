# build-hook

Small Rust service that exposes a build hook API over HTTP using [axum](https://docs.rs/axum/latest/axum/index.html).
It authenticates requests with bearer tokens and builds container images
through Docker `buildx` using the remote driver connected to an
external [BuildKit](https://docs.docker.com/build/buildkit/configure/) daemon.

- Fills the role of GitHub actions on pushes to main, but for free.
- Triggers image builds and resource deployments for configured projects on request.
- Supports multiple projects, each able to build multiple images and restart multiple Kubernetes services.

## Requirements

- Rust `stable` toolchain (edition 2024)
- Docker with `buildx` configured to reach a BuildKit daemon

## Configuration

- `config.toml` is required at startup and holds runtime settings.
- `BEARER_TOKENS` is a comma-separated list of valid bearer tokens _(no particular format)_.

### config.toml format

```toml
[app]
registry = "registry.example.com"

[[projects]]
name = "My Web Application"
slug = "my-app"

[projects.code]
url = "https://github.com/example/my-app"
branch = "main"

[[projects.image]]
repository = "my-app/web"
location = "Dockerfile"
tag = "latest"

[[projects.image]]
repository = "my-app/api"
location = "api/Dockerfile"
tag = "release"

[projects.deployments]
namespace = "app"
resources = ["deployment/web", "deployment/api"]
```

### Configuration Breakdown

**NOTE**: `config.toml` is expected in the project root, which is either `/app` when running in containers or the repository root when running locally.

#### App

- `app.registry`: Base image registry hostname used to tag images (for example `ghcr.io/org`).
- `projects`: List of projects to build and restart.

#### Project

- `projects.name`: Display name for the project.
- `projects.slug`: Unique slug used for routing at `/{slug}` and local clone paths.
- `projects.image`: One or more images to build per project.

#### Source Code

- `projects.code.url`: HTTPS Git repository URL (public or private).
- `projects.code.branch`: Branch to build from.

#### Images

- `projects.image.repository`: Repository path under the registry (for example `org/app`).
- `projects.image.location`: Dockerfile path relative to the repo root (no `..` segments).
- `projects.image.tag`: Tag to apply to the image.

#### Deployments

- `projects.deployments.namespace`: Kubernetes namespace for rollout restarts.
- `projects.deployments.resources`: Kubernetes resources to restart (format: `type/name`).

_Rust Docs page coming soon..._
