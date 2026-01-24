use std::collections::HashMap;

use serde::Deserialize;

use crate::project::ProjectConfig;

#[derive(Debug, Deserialize)]
pub struct ConfigFile {
    app: AppConfig,
    projects: Vec<ProjectConfig>,
}

#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub registry: String,
}

pub struct HookConfig {
    pub app: AppConfig,
    pub projects: HashMap<String, ProjectConfig>,
}

pub fn load() -> Result<HookConfig, String> {
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

fn validate(config: &ConfigFile) -> Result<(), String> {
    // app.registry should not be empty
    if config.app.registry.trim().is_empty() {
        return Err("`app.registry` must not be empty!".to_string());
    }

    // app.registry should be a valid URL (http/https allowed for registries)
    validate_registry_url(&config.app.registry)
        .map_err(|_| "`app.registry` must be a valid URL!".to_string())?;

    for project in &config.projects {
        project.validate()?;
    }

    Ok(())
}

fn validate_registry_url(url: &str) -> Result<(), String> {
    // Must start with http:// or https://
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("Registry URL must start with http:// or https://".to_string());
    }

    // After scheme, there should be at least one character (host)
    let scheme_len = if url.starts_with("https://") { 8 } else { 7 };
    let after_scheme = &url[scheme_len..];
    if after_scheme.is_empty() {
        return Err("Registry URL must have a host".to_string());
    }

    // Basic validation: should have a host part
    let host_part = if let Some(slash_pos) = after_scheme.find('/') {
        &after_scheme[..slash_pos]
    } else {
        after_scheme
    };

    if host_part.is_empty() {
        return Err("Registry URL must have a host".to_string());
    }

    // Basic character validation for host (alphanumeric, dots, hyphens, colons)
    if !host_part
        .chars()
        .all(|c| c.is_alphanumeric() || c == '.' || c == '-' || c == ':' || c == '[' || c == ']')
    {
        return Err("Registry URL contains invalid characters in host".to_string());
    }

    Ok(())
}

fn log(config: &HookConfig) {
    tracing::debug!("Image registry: {}", config.app.registry);
    tracing::debug!("Loaded {} project(s):", config.projects.len());

    for project in config.projects.values() {
        project.log();
    }
}
