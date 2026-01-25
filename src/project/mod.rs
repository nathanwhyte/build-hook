mod image;
mod repo;

use crate::kube;
use serde::Deserialize;
use std::path::{Component, Path};

#[derive(Debug, Deserialize)]
pub struct ProjectConfig {
    name: String,
    slug: String,
    code: CodeConfig,
    image: Vec<ImageConfig>,
    deployments: DeploymentConfig,
}

#[derive(Debug, Deserialize)]
pub struct CodeConfig {
    url: String,
    branch: String,
}

#[derive(Debug, Deserialize)]
pub struct ImageConfig {
    repository: String,
    location: String,
    tag: String,
}

#[derive(Debug, Deserialize)]
pub struct DeploymentConfig {
    namespace: String,
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
        let code_url = self.code.url.clone();
        let code_branch = self.code.branch.clone();
        let image_specs: Vec<(String, String, String)> = self
            .image
            .iter()
            .map(|image| {
                (
                    image.repository.clone(),
                    image.location.clone(),
                    image.tag.clone(),
                )
            })
            .collect();

        let namespace = self.deployments.namespace.clone();
        let resources = self.deployments.resources.clone();
        let slug = self.slug.clone();
        let registry = registry.to_string();
        let github_token = github_token.to_string();

        tokio::task::spawn_blocking(move || {
            if let Err(err) = repo::clone_repo(&github_token, &code_url, &repo_dest, &code_branch) {
                tracing::error!("Failed to clone repository for project `{}`: {}", slug, err);
                return;
            }

            let image_builds: Vec<image::BuildImage> = image_specs
                .into_iter()
                .map(|(repository, location, tag)| {
                    let image_tag = format!("{}/{}:{}", registry, repository, tag);
                    let dockerfile_path = Path::new(&repo_dest).join(&location);
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

            match image::build_images(image_builds, repo_dest) {
                Ok(()) => {
                    if let Err(e) = kube::rollout_restart(&namespace, &resources) {
                        tracing::error!("Rollout restart failed for project `{}`: {}", slug, e);
                    }
                }
                Err(e) => {
                    tracing::error!("Build failed for project `{}`: {}", slug, e);
                }
            }
        });
        Ok(())
    }

    pub fn slug(&self) -> &str {
        &self.slug
    }
}
