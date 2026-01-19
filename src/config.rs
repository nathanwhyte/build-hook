use std::collections::HashMap;

use serde::Deserialize;
use url::Url;

#[derive(Debug, Deserialize)]
pub struct ConfigFile {
    app: AppConfig,
    projects: Vec<ProjectConfig>,
}

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    registry: String,
    cache: bool,
}

#[derive(Debug, Deserialize)]
pub struct ProjectConfig {
    pub name: String,
    pub slug: String,
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

pub type Config = HashMap<String, ProjectConfig>;

pub fn load() -> Result<Config, String> {
    // read projects config file
    let file_string = match std::fs::read_to_string("config.toml") {
        Ok(contents) => contents,
        Err(_) => panic!("Could not read config.toml file!"),
    };

    let config_file: ConfigFile = match toml::from_str(&file_string) {
        Ok(v) => v,
        Err(e) => panic!("Could not parse config.toml file: {}", e),
    };

    validate(&config_file)?;

    let mut config: Config = HashMap::new();

    config_file.projects.into_iter().for_each(|project| {
        config.insert(project.slug.clone(), project);
    });

    log(&config_file.app, &config);

    Ok(config)
}

fn validate(config: &ConfigFile) -> Result<(), String> {
    // app.registry should be a valid HTTPS URL
    validate_url(&config.app.registry, "app.registry")?;

    // app.cache is a boolean, no need to validate

    for project in &config.projects {
        // project.name should not be empty
        if project.name.trim().is_empty() {
            return Err("project.name must not be empty!".to_string());
        }

        if project.slug.trim().is_empty() {
            return Err("project.slug must not be empty!".to_string());
        }

        // project.code.url should be a valid URL
        validate_url(&project.code.url, "project.code.url")?;

        // project.code.branch should not be empty
        if project.code.branch.trim().is_empty() {
            return Err("project.code.branch must not be empty!".to_string());
        }

        // project.code.public is a boolean, no need to validate

        // project.image.repository should not be empty
        if project.image.repository.trim().is_empty() {
            return Err("project.image.repository must not be empty!".to_string());
        }

        // project.image.tag should not be empty
        if project.image.tag.trim().is_empty() {
            return Err("project.image.tag must not be empty!".to_string());
        }

        // project.deployments.namespace should not be empty
        if project.deployments.namespace.trim().is_empty() {
            return Err("project.deployments.namespace must not be empty!".to_string());
        }

        // need at least 1 item specified in project.deployments.resources
        if project.deployments.resources.is_empty() {
            return Err("project.deployments.resources must have at least one item!".to_string());
        }
    }

    Ok(())
}

fn validate_url(url: &str, field: &str) -> Result<(), String> {
    let url = Url::parse(url).map_err(|_| format!("`{}` must be a valid URL!", field))?;

    if url.scheme() != "https" {
        return Err(format!("`{}` must use HTTPS!", field));
    }

    Ok(())
}

fn log(app_config: &AppConfig, config_map: &Config) {
    tracing::debug!("Builds should be cached: {}", app_config.cache);

    tracing::debug!("Loaded {} project(s):", config_map.len());

    for (slug, project) in config_map {
        tracing::debug!("---");
        tracing::debug!("Project: {}, {}", project.name, slug);
        tracing::debug!("  Code URL: {}", project.code.url);
        tracing::debug!("  Code Branch: {}", project.code.branch);
        tracing::debug!("  Code is Public: {}", project.code.public);
        tracing::debug!("  Image Repository: {}", project.image.repository);
        tracing::debug!("  Image Tag: {}", project.image.tag);
        tracing::debug!("  Deployment Namespace: {}", project.deployments.namespace);
        tracing::debug!(
            "  Deployment Resources: {:?}",
            project.deployments.resources
        );
    }
}
