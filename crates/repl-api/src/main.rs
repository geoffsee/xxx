use axum::{
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    println!("repl-api server starting...");

    let app = Router::new()
        .route("/api/repl/execute", post(repl_api::execute_repl))
        .route("/api/repl/languages", get(repl_api::list_languages));

    let addr = SocketAddr::from(([0, 0, 0, 0], 3001));
    println!("repl-api listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
