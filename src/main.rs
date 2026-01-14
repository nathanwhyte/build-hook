use std::env;

use axum::{Router, routing::post};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Config {
    app: AppConfig,
    projects: Vec<ProjectConfig>,
}

#[derive(Debug, Deserialize)]
struct AppConfig {
    registry: String,
}

#[derive(Debug, Deserialize)]
struct ProjectConfig {
    name: String,
    trigger: String,
}

#[tokio::main]
async fn main() {
    // read in comma-separated list of bearer tokens from env
    let tokens_string = match env::var("BEARER_TOKENS") {
        Ok(tokens) => tokens,
        Err(_) => panic!("No BEARER_TOKENS environment variable set!"),
    };

    let tokens_list = tokens_string
        .split(',')
        .map(|s| s.trim().to_string())
        .collect::<Vec<String>>();

    println!("Loaded {} bearer tokens", tokens_list.len());

    // read projects config file
    let config = std::fs::read_to_string("config.toml").unwrap();
    let value: Config = toml::from_str(&config).unwrap();

    println!("Registry: {:?}", value.app.registry);

    for project in value.projects {
        println!("Project: {} -> {}", project.name, project.trigger);
    }

    // build our application with a single route
    let app = Router::new().route("/", post(|| async { "Hello, World!" }));

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
