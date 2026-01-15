mod config;

#[tokio::main]
async fn main() {
    // read in comma-separated list of bearer tokens from env
    // let tokens_string = match env::var("BEARER_TOKENS") {
    //     Ok(tokens) => tokens,
    //     Err(_) => panic!("No BEARER_TOKENS environment variable set!"),
    // };
    //
    // let tokens_list = tokens_string
    //     .split(',')
    //     .map(|s| s.trim().to_string())
    //     .collect::<Vec<String>>();

    // read in env and config
    let _config = match config::load() {
        Ok(cfg) => cfg,
        Err(e) => panic!("Could not load config: {}", e),
    };

    // build our application with a single route
    // let app = Router::new().route("/", post(|| async { "Hello, World!" }));

    // run our app with hyper, listening globally on port 3000
    // let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    // axum::serve(listener, app).await.unwrap();
}
