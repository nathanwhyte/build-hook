mod image;
mod repo;

use crate::kube;
use serde::Deserialize;
use std::path::{Component, Path};

/// Configuration for a buildable project.
#[derive(Clone, Debug, Deserialize)]
pub struct ProjectConfig {
    /// Human-friendly project name.
    name: String,
    /// Unique project slug used for routing.
    /// Router expects requests at "/{slug}", e.g. https://build-hook.example.com/my-project
    slug: String,
    /// Source repository configuration.
    /// Can be public or private, but private repos require setting up a GitHub token.
    code: CodeConfig,
    /// Container image build definitions.
    /// Supports one or more images to be built from the same repository.
    image: Vec<ImageConfig>,
    /// Kubernets deployment targets to restart after builds succeed.
    deployments: DeploymentConfig,
}

/// Code repository settings for a project.
#[derive(Clone, Debug, Deserialize)]
pub struct CodeConfig {
    /// URL (https) to the project's GitHub repository.
    url: String,
    /// Target branch to pull code from.
    branch: String,
}

/// Image build configuration for a project.
#[derive(Clone, Debug, Deserialize)]
pub struct ImageConfig {
    /// Repository path under the configured registry.
    /// e.g. "my-org/my-app" for an image tagged as "gcr.io/my-org/my-app:latest"
    repository: String,
    /// Dockerfile path relative to the repo root.
    ///
    /// If the Dockerfile is in the repo root, this should be just "Dockerfile".
    /// If it's in a subdirectory, specify the relative path, e.g. "services/api/Dockerfile".
    location: String,
    /// Tag applied to the built image, e.g. "latest" or "v1.2.3".
    tag: String,
}

/// Kubernetes deployment restart configuration.
#[derive(Clone, Debug, Deserialize)]
pub struct DeploymentConfig {
    /// Kubernetes namespace where the resources are deployed.
    namespace: String,
    /// Array of resource names to restart once builds succeed.
    ///
    /// Must be written as "<resource_type>/<resource_name>", e.g. "deployment/my-app" or "statefulset/my-db".
    resources: Vec<String>,
}

fn validate_https_url(url: &str) -> Result<(), String> {
    // Must start with https://
    if !url.starts_with("https://") {
        return Err("URL must start with https://".to_string());
    }

    // After https://, there should be at least one character (host)
    let after_scheme = &url[8..];
    if after_scheme.is_empty() {
        return Err("URL must have a host".to_string());
    }

    // Extract host part (everything before the first /)
    let host_part = if let Some(slash_pos) = after_scheme.find('/') {
        &after_scheme[..slash_pos]
    } else {
        after_scheme
    };

    if host_part.is_empty() {
        return Err("URL must have a host".to_string());
    }

    // Basic character validation for host (alphanumeric, dots, hyphens, colons for ports, brackets for IPv6)
    if !host_part
        .chars()
        .all(|c| c.is_alphanumeric() || c == '.' || c == '-' || c == ':' || c == '[' || c == ']')
    {
        return Err("URL contains invalid characters in host".to_string());
    }

    Ok(())
}

impl ProjectConfig {
    pub fn validate(&self) -> Result<(), String> {
        // project.name should not be empty
        if self.name.trim().is_empty() {
            return Err("project.name must not be empty!".to_string());
        }

        if self.slug.trim().is_empty() {
            return Err("project.slug must not be empty!".to_string());
        }

        // project.code.url should be a valid HTTPS URL
        validate_https_url(&self.code.url)
            .map_err(|_| "`project.code.url` must be a valid HTTPS URL!".to_string())?;

        // project.code.branch should not be empty
        if self.code.branch.trim().is_empty() {
            return Err("project.code.branch must not be empty!".to_string());
        }

        if self.image.is_empty() {
            return Err("project.image must have at least one entry!".to_string());
        }

        for image in &self.image {
            // project.image.repository should not be empty
            if image.repository.trim().is_empty() {
                return Err("project.image.repository must not be empty!".to_string());
            }

            // project.image.location should not be empty
            if image.location.trim().is_empty() {
                return Err("project.image.location must not be empty!".to_string());
            }

            let location_path = Path::new(&image.location);
            if location_path.is_absolute() {
                return Err("project.image.location must be a relative path!".to_string());
            }
            if location_path
                .components()
                .any(|component| matches!(component, Component::ParentDir))
            {
                return Err("project.image.location must not contain parent paths!".to_string());
            }

            // project.image.tag should not be empty
            if image.tag.trim().is_empty() {
                return Err("project.image.tag must not be empty!".to_string());
            }
        }

        // project.deployments.namespace should not be empty
        if self.deployments.namespace.trim().is_empty() {
            return Err("project.deployments.namespace must not be empty!".to_string());
        }

        // need at least 1 item specified in project.deployments.resources
        if self.deployments.resources.is_empty() {
            return Err("project.deployments.resources must have at least one item!".to_string());
        }

        Ok(())
    }

    pub fn build(&self, registry: &str, github_token: &str) -> Result<(), String> {
        let repo_dest = format!("/tmp/{}", self.slug);
        repo::clone_repo(github_token, &self.code.url, &repo_dest, &self.code.branch)
            .map_err(|err| format!("Failed to clone repository: {}", err))?;

        let image_builds: Vec<image::BuildImage> = self
            .image
            .iter()
            .map(|image| {
                let image_tag = format!("{}/{}:{}", registry, image.repository, image.tag);
                let dockerfile_path = Path::new(&repo_dest).join(&image.location);
                let context_dir = dockerfile_path
                    .parent()
                    .unwrap_or_else(|| Path::new(&repo_dest))
                    .to_string_lossy()
                    .to_string();
                image::BuildImage {
                    tag: image_tag,
                    dockerfile_path: dockerfile_path.to_string_lossy().to_string(),
                    context_dir,
                }
            })
            .collect();

        image::build_images(image_builds, repo_dest)?;
        kube::rollout_restart(&self.deployments.namespace, &self.deployments.resources)?;
        Ok(())
    }

    pub fn slug(&self) -> &str {
        &self.slug
    }
}
