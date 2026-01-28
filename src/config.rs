use std::collections::HashMap;

use serde::Deserialize;

use crate::project::ProjectConfig;

/// Raw config file model parsed from config.toml.
#[derive(Debug, Deserialize)]
pub struct ConfigFile {
    /// Global app configuration.
    app: AppConfig,
    /// Project definitions loaded from config.toml.
    projects: Vec<ProjectConfig>,
}

/// Application-level settings loaded from config.toml.
#[derive(Debug, Deserialize)]
pub struct AppConfig {
    /// Base image registry hostname used to tag images.
    /// Can be any valid container registry (e.g., Docker Hub, ECR, GCR).
    /// Examples: "ghcr.io/my-org", "123456789012.dkr.ecr.us-west-2.amazonaws.com/my-repo"
    pub registry: String,
}

/// Runtime configuration parsed from `config.toml`.
/// Used in shared application state.
pub struct HookConfig {
    /// Application-wide configuration.
    pub app: AppConfig,
    /// Project configs keyed by slug for quick lookup.
    pub projects: HashMap<String, ProjectConfig>,
}

/// Load and validate configuration from `config.toml`.
///
/// Expects `config.toml` to be in the current working directory, which is `/app` when running
/// in containers or the project root when running locally.
pub fn load() -> Result<HookConfig, String> {
    // read projects config file
    let file_string = std::fs::read_to_string("config.toml")
        .map_err(|_| "Could not read config.toml file!".to_string())?;

    let config_file: ConfigFile = toml::from_str(&file_string)
        .map_err(|e| format!("Could not parse config.toml file: {}", e))?;

    validate(&config_file)?;

    let mut config: HashMap<String, ProjectConfig> = HashMap::new();

    config_file.projects.into_iter().for_each(|project| {
        config.insert(project.slug().to_owned(), project);
    });

    let config = HookConfig {
        app: config_file.app,
        projects: config,
    };

    log(&config);

    Ok(config)
}

/// Fail-fast validation of the loaded configuration.
fn validate(config: &ConfigFile) -> Result<(), String> {
    // app.registry should not be empty
    if config.app.registry.trim().is_empty() {
        return Err("`app.registry` must not be empty!".to_string());
    }

    for project in &config.projects {
        project.validate()?;
    }

    Ok(())
}

/// Emit debug logs about loaded configuration.
fn log(config: &HookConfig) {
    tracing::info!("Configured image registry: {}", config.app.registry);
    tracing::info!("Loaded {} project(s):", config.projects.len());
}
