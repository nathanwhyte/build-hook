mod repo;

use serde::Deserialize;
use std::path::{Component, Path};
use url::Url;

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

    pub fn build(&self, cache: bool, registry: &str) -> Result<(), String> {
        let repo_dest = match cache {
            true => format!("/cache/{}", self.slug),
            false => format!("/tmp/{}", self.slug),
        };

        // Clone repository
        repo::clone_repo(&self.code.url, &repo_dest, &self.code.branch);

        let mut image_builds: Vec<BuildImage> = self
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
                BuildImage {
                    tag,
                    dockerfile_path: dockerfile_path.to_string_lossy().to_string(),
                    context_dir,
                }
            })
            .collect();

        for build in &image_builds {
            if !Path::new(&build.dockerfile_path).is_file() {
                return Err(format!(
                    "Dockerfile for {} not found at {}",
                    build.tag, build.dockerfile_path
                ));
            }
        }

        let first_build = image_builds
            .drain(0..1)
            .next()
            .ok_or_else(|| "project.image must have at least one entry!".to_string())?;

        tracing::info!(
            "building {} using {}",
            first_build.tag,
            first_build.dockerfile_path
        );

        let mut child = spawn_build(
            &first_build.context_dir,
            &first_build.tag,
            &first_build.dockerfile_path,
        )?;
        verify_build_started(&mut child)?;

        // Spawn background task to handle build completion
        std::thread::spawn(move || {
            handle_build_completion(child, first_build.tag);

            for build in image_builds {
                tracing::info!("building {} using {}", build.tag, build.dockerfile_path);
                match spawn_build(&build.context_dir, &build.tag, &build.dockerfile_path) {
                    Ok(mut next_child) => {
                        if let Err(e) = verify_build_started(&mut next_child) {
                            tracing::error!(
                                "Build process for {} exited immediately: {}",
                                build.tag,
                                e
                            );
                            break;
                        }
                        handle_build_completion(next_child, build.tag);
                    }
                    Err(e) => {
                        tracing::error!("Failed to start build for {}: {}", build.tag, e);
                        break;
                    }
                }
            }

            if !cache {
                // If not caching, remove the cloned repository after the builds
                if let Err(e) = std::fs::remove_dir_all(&repo_dest) {
                    tracing::warn!(
                        "Failed to remove temporary repository directory {}: {}",
                        repo_dest,
                        e
                    );
                }
            }
        });

        Ok(())
    }

    pub fn slug(&self) -> &str {
        &self.slug
    }
}

struct BuildImage {
    tag: String,
    dockerfile_path: String,
    context_dir: String,
}

fn spawn_build(
    context_dir: &str,
    image_tag: &str,
    dockerfile_path: &str,
) -> Result<std::process::Child, String> {
    std::process::Command::new("docker")
        .args([
            "buildx",
            "build",
            "--builder",
            "builder",
            "--platform",
            "linux/amd64",
            "--push",
            "-t",
            image_tag,
            "--file",
            dockerfile_path,
            context_dir,
        ])
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()
        .map_err(|e| format!("Failed to execute docker buildx: {}", e))
}

fn verify_build_started(child: &mut std::process::Child) -> Result<(), String> {
    match child.try_wait() {
        Ok(Some(status)) => {
            if !status.success() {
                return Err(format!(
                    "Build process exited immediately with code: {:?}",
                    status.code()
                ));
            }
        }
        Ok(None) => {}
        Err(e) => {
            return Err(format!("Failed to check build process status: {}", e));
        }
    }

    Ok(())
}

fn handle_build_completion(child: std::process::Child, image_tag: String) {
    let output = match child.wait_with_output() {
        Ok(output) => output,
        Err(e) => {
            tracing::error!("Failed to wait for build process: {}", e);
            return;
        }
    };

    if !output.stdout.is_empty() {
        tracing::debug!(
            "Build stdout for {}: {}",
            image_tag,
            String::from_utf8_lossy(&output.stdout)
        );
    }
    if !output.status.success() && !output.stderr.is_empty() {
        tracing::warn!(
            "Build stderr for {}: {}",
            image_tag,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    if !output.status.success() {
        tracing::error!(
            "Build failed for {} with exit code: {:?}",
            image_tag,
            output.status.code()
        );
    } else {
        tracing::info!("Successfully built and pushed image: {}", image_tag);
    }
}
