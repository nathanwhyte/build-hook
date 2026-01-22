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
    pub cache: bool,
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

    // app.cache is a boolean, no need to validate

    for project in &config.projects {
        project.validate()?;
    }

    Ok(())
}

fn log(config: &HookConfig) {
    tracing::debug!("Image registry: {}", config.app.registry);
    tracing::debug!("Builds should be cached: {}", config.app.cache);

    tracing::debug!("Loaded {} project(s):", config.projects.len());

    for project in config.projects.values() {
        project.log();
    }
}
