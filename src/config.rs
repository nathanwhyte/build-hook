use serde::Deserialize;
use url::Url;

#[derive(Debug, Deserialize)]
struct Config {
    app: AppConfig,
    project: Vec<ProjectConfig>,
}

#[derive(Debug, Deserialize)]
struct AppConfig {
    registry: String,
}

#[derive(Debug, Deserialize)]
struct ProjectConfig {
    name: String,
    code: CodeConfig,
    image: ImageConfig,
    deployments: DeploymentConfig,
}

#[derive(Debug, Deserialize)]
struct CodeConfig {
    url: String,
    branch: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ImageConfig {
    repository: String,
    tag: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DeploymentConfig {
    namespace: String,
    resources: Vec<String>,
}

pub fn load() {
    // read projects config file
    let config = match std::fs::read_to_string("config.toml") {
        Ok(contents) => contents,
        Err(_) => panic!("Could not read config.toml file!"),
    };

    let value: Config = match toml::from_str(&config) {
        Ok(v) => v,
        Err(e) => panic!("Could not parse config.toml file: {}", e),
    };

    // TODO: validate config

    // match Url::parse(&value.app.registry) {
    //     Ok(url) => match url.is_special() {
    //         true => (),
    //         false => panic!("Registry URL must start with https://"),
    //     },
    //     Err(_) => panic!("Invalid URL in `app.registry`"),
    // }

    println!("---");
    println!("Image Registry Base URL: {}", value.app.registry);

    for project in value.project {
        println!("\n---");
        println!("Project: {}", project.name);
        println!("  Code URL: {}", project.code.url);
        println!(
            "  Code Branch: {}",
            project.code.branch.unwrap_or("main".to_string())
        );
        println!("  Image Repository: {}", project.image.repository);
        println!(
            "  Image Tag: {}",
            project.image.tag.unwrap_or("latest".to_string())
        );
        println!("  Deployment Namespace: {}", project.deployments.namespace);
        println!(
            "  Deployment Resources: {:?}",
            project.deployments.resources
        );
    }
}
