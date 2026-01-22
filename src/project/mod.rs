mod repo;

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

    pub fn build(&self, cache: bool, registry: &str) -> Result<(), String> {
        let repo_dest = match cache {
            true => format!("/cache/{}", self.slug),
            false => format!("/tmp/{}", self.slug),
        };

        // Clone repository
        repo::clone_repo(&self.code.url, &repo_dest, &self.code.branch);

        // Construct image tag: <registry>/<slug>/<repository>:<tag>
        let image_tag = format!(
            "{}/{}/{}:{}",
            registry, self.slug, self.image.repository, self.image.tag
        );

        tracing::info!("Building and pushing image: {}", image_tag);
        tracing::info!("Build context: {}", repo_dest);
        tracing::info!(
            "Executing: docker buildx build --builder builder --platform linux/amd64 --push -t {} {}",
            image_tag,
            repo_dest
        );

        // Execute docker buildx build --push with amd64 platform
        let output = std::process::Command::new("docker")
            .args(&[
                "buildx",
                "build",
                "--builder",
                "builder",
                "--platform",
                "linux/amd64",
                "--push",
                "-t",
                &image_tag,
                &repo_dest,
            ])
            .output()
            .map_err(|e| format!("Failed to execute docker buildx: {}", e))?;

        // Log stdout line by line for better visibility
        let stdout = String::from_utf8_lossy(&output.stdout);
        if !stdout.trim().is_empty() {
            for line in stdout.lines() {
                tracing::debug!("[build] {}", line);
            }
        }

        // Log stderr line by line
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.trim().is_empty() {
            for line in stderr.lines() {
                tracing::debug!("[build] {}", line);
            }
        }

        if !output.status.success() {
            tracing::error!(
                "Build failed for {} with exit code: {:?}",
                image_tag,
                output.status.code()
            );
            return Err(format!("Build failed: {}", stderr));
        }

        tracing::info!("Successfully built and pushed image: {}", image_tag);
        Ok(())
    }

    pub fn slug(&self) -> &str {
        &self.slug
    }
}
