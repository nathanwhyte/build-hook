mod config;

#[tokio::main]
async fn main() {
    // read in comma-separated list of bearer tokens from env
    let _bearer_tokens = load_env();

    // read in env and config
    let _config = match config::load() {
        Ok(cfg) => cfg,
        Err(e) => panic!("Could not load config: {}", e),
    };

    let output = std::process::Command::new("kubectl")
        .arg("version")
        .output()
        .expect("Failed to execute command");

    println!("\n---");
    println!("Output: {}", String::from_utf8_lossy(&output.stdout));

    // build our application with a single route
    // let app = Router::new().route("/", post(|| async { "Hello, World!" }));

    // run our app with hyper, listening globally on port 3000
    // let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    // axum::serve(listener, app).await.unwrap();
}

fn load_env() -> Vec<String> {
    // NOTE: temporary, bacon doesn't support env in `run` jobs
    let tokens_string = std::env::var("BEARER_TOKENS").unwrap_or("token12345".to_string());

    // let tokens_string = match std::env::var("BEARER_TOKENS") {
    //     Ok(tokens) => tokens,
    //     Err(_) => panic!("No BEARER_TOKENS environment variable set!"),
    // };

    tokens_string
        .split(',')
        .map(|s| s.trim().to_string())
        .collect::<Vec<String>>()
}
