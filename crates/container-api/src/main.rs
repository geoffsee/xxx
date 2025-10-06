use axum::{Router, routing::get};
use container_api::{create_container, create_container_stream, health, list_containers, remove_container};
use service_registry::register_service;
use tower_http::trace::TraceLayer;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    tracing_subscriber::fmt::init();

    // Register service with etcd
    let (service, _lease_id) = register_service!("container-api", "container-api", 3000).await;
    tracing::info!("Service registered: {} ({})", service.name, service.id);

    let app = Router::new()
        .route("/healthz", get(health))
        .route("/api/containers/list", get(list_containers))
        .route(
            "/api/containers/create",
            axum::routing::post(create_container),
        )
        .route(
            "/api/containers/create/stream",
            axum::routing::post(create_container_stream),
        )
        .route(
            "/api/containers",
            axum::routing::delete(remove_container),
        )
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Server listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}