use std::env;

use axum::{Router, routing::post};

#[tokio::main]
async fn main() {
    // build our application with a single route
    let app = Router::new().route("/", post(|| async { "Hello, World!" }));

    let tokens_string = match env::var("BEARER_TOKENS") {
        Ok(tokens) => tokens,
        Err(_) => panic!("No BEARER_TOKENS environment variable set!"),
    };

    let tokens_list = tokens_string
        .split(',')
        .map(|s| s.trim().to_string())
        .collect::<Vec<String>>();

    println!("Loaded {} bearer tokens", tokens_list.len());

    // TODO: one POST router per configured project

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
