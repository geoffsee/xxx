use axum::{
    routing::{get, post},
    Router,
};
use service_registry::register_service;
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("repl-api server starting...");

    // Register service with etcd
    let (service, _lease_id) = register_service!("repl-api", "repl-api", 3000).await;
    tracing::info!("Service registered: {} ({})", service.name, service.id);

    let app = Router::new()
        .route("/api/repl/execute", post(repl_api::execute_repl))
        .route("/api/repl/execute/stream", post(repl_api::execute_repl_stream))
        .route("/api/repl/languages", get(repl_api::list_languages));

    let addr = SocketAddr::from(([0, 0, 0, 0], 3001));
    println!("repl-api listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
