mod repo;

use std::path::Path;

use serde::Deserialize;
use url::Url;

#[derive(Debug, Deserialize)]
pub struct ProjectConfig {
    name: String,
    slug: String,
    code: CodeConfig,
    image: ImageConfig,
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
    tag: String,
}

#[derive(Debug, Deserialize)]
pub struct DeploymentConfig {
    namespace: String,
    resources: Vec<String>,
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

        // project.code.url should be a valid URL
        let code_url = Url::parse(&self.code.url)
            .map_err(|_| "`project.code.url` must be a valid URL!".to_string())?;
        if code_url.scheme() != "https" {
            return Err("`project.code.url` must use HTTPS!".to_string());
        }

        // project.code.branch should not be empty
        if self.code.branch.trim().is_empty() {
            return Err("project.code.branch must not be empty!".to_string());
        }

        // project.code.public is a boolean, no need to validate

        // project.image.repository should not be empty
        if self.image.repository.trim().is_empty() {
            return Err("project.image.repository must not be empty!".to_string());
        }

        // project.image.tag should not be empty
        if self.image.tag.trim().is_empty() {
            return Err("project.image.tag must not be empty!".to_string());
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
        tracing::debug!("  Image Repository: {}", self.image.repository);
        tracing::debug!("  Image Tag: {}", self.image.tag);
        tracing::debug!("  Deployment Namespace: {}", self.deployments.namespace);
        tracing::debug!("  Deployment Resources: {:?}", self.deployments.resources);
    }

    pub fn build(&self, cache: bool) {
        let repo_dest = match cache {
            true => format!("/cache/{}", self.slug),
            false => format!("/tmp/{}", self.slug),
        };

        let _repo = repo::clone(&self.code.url, &repo_dest);
    }

    pub fn slug(&self) -> &str {
        &self.slug
    }
}
