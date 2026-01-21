use std::collections::HashMap;

use serde::Deserialize;
use url::Url;

use crate::project::ProjectConfig;

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
        config.insert(project.slug().to_owned(), project);
    });

    log(&config_file.app, &config);

    Ok(config)
}

fn validate(config: &ConfigFile) -> Result<(), String> {
    // app.registry should be a valid HTTPS URL
    let registry_url = Url::parse(&config.app.registry)
        .map_err(|_| "`app.registry` must be a valid URL!".to_string())?;
    if registry_url.scheme() != "https" {
        return Err("`app.registry` must use HTTPS!".to_string());
    }

    // app.cache is a boolean, no need to validate

    for project in &config.projects {
        project.validate()?;
    }

    Ok(())
}

fn log(app_config: &AppConfig, config_map: &Config) {
    tracing::debug!("Image registry: {}", app_config.registry);
    tracing::debug!("Builds should be cached: {}", app_config.cache);

    tracing::debug!("Loaded {} project(s):", config_map.len());

    for project in config_map.values() {
        project.log();
    }
}
