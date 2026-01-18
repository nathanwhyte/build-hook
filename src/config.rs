use serde::Deserialize;
use url::Url;

#[derive(Debug, Deserialize)]
pub struct Config {
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
    name: String,
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

pub fn load() -> Result<Config, String> {
    // read projects config file
    let file_string = match std::fs::read_to_string("config.toml") {
        Ok(contents) => contents,
        Err(_) => panic!("Could not read config.toml file!"),
    };

    let config: Config = match toml::from_str(&file_string) {
        Ok(v) => v,
        Err(e) => panic!("Could not parse config.toml file: {}", e),
    };

    validate(&config)?;

    log(&config);

    Ok(config)
}

fn validate(config: &Config) -> Result<(), String> {
    // app.registry should be a valid HTTPS URL
    validate_url(&config.app.registry, "app.registry")?;

    for project in &config.projects {
        // project.name should not be empty
        if project.name.trim().is_empty() {
            return Err("project.name must not be empty!".to_string());
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

fn log(value: &Config) {
    println!("---");
    println!("Image Registry URL: {}", value.app.registry);
    println!("Cache Builds: {}", value.app.cache);

    for project in &value.projects {
        println!("\n---");
        println!("Project: {}", project.name);
        println!("  Code URL: {}", project.code.url);
        println!("  Code Branch: {}", project.code.branch);
        println!("  Code is Public: {}", project.code.public);
        println!("  Image Repository: {}", project.image.repository);
        println!("  Image Tag: {}", project.image.tag);
        println!("  Deployment Namespace: {}", project.deployments.namespace);
        println!(
            "  Deployment Resources: {:?}",
            project.deployments.resources
        );
    }
}
