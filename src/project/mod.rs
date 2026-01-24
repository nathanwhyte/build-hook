mod image;
mod repo;

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
    public: bool,
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

        // project.code.public is a boolean, no need to validate

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

    pub fn log(&self) {
        tracing::debug!("---");
        tracing::debug!("Project: {}, {}", self.name, self.slug);
        tracing::debug!("  Code URL: {}", self.code.url);
        tracing::debug!("  Code Branch: {}", self.code.branch);
        tracing::debug!("  Code is Public: {}", self.code.public);
        tracing::debug!("  Images: {}", self.image.len());
        for image in &self.image {
            tracing::debug!(
                "  Image: {}:{} (Dockerfile: {})",
                image.repository,
                image.tag,
                image.location
            );
        }
        tracing::debug!("  Deployment Namespace: {}", self.deployments.namespace);
        tracing::debug!("  Deployment Resources: {:?}", self.deployments.resources);
    }

    pub fn build(&self, registry: &str, github_token: &str) -> Result<(), String> {
        let repo_dest = format!("/tmp/{}", self.slug);

        // Clone repository
        repo::clone_repo(github_token, &self.code.url, &repo_dest, &self.code.branch)
            .map_err(|err| format!("Failed to clone repository: {}", err))?;

        let image_builds: Vec<image::BuildImage> = self
            .image
            .iter()
            .map(|image| {
                let tag = format!("{}/{}:{}", registry, image.repository, image.tag);
                let dockerfile_path = Path::new(&repo_dest).join(&image.location);
                let context_dir = dockerfile_path
                    .parent()
                    .unwrap_or_else(|| Path::new(&repo_dest))
                    .to_string_lossy()
                    .to_string();
                image::BuildImage {
                    tag,
                    dockerfile_path: dockerfile_path.to_string_lossy().to_string(),
                    context_dir,
                }
            })
            .collect();

        image::build_images(image_builds, repo_dest)

        // TODO: restart k8s services
    }

    pub fn slug(&self) -> &str {
        &self.slug
    }
}
