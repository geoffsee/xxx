use axum::{
    Router,
    routing::{get, post},
};
use service_registry::api;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::trace::TraceLayer;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // Get etcd endpoints from environment
    let etcd_endpoints = std::env::var("ETCD_ENDPOINTS")
        .unwrap_or_else(|_| "localhost:2379".to_string())
        .split(',')
        .map(|s| s.to_string())
        .collect::<Vec<_>>();

    tracing::info!("Connecting to etcd at: {:?}", etcd_endpoints);

    // Create registry
    let mut registry = service_registry::ServiceRegistry::new(etcd_endpoints, Some(10))
        .await
        .expect("Failed to connect to etcd");

    // Auto-register CoreOS if COREOS_URL is set
    if let Ok(coreos_url) = std::env::var("COREOS_URL") {
        tracing::info!("Auto-registering CoreOS from COREOS_URL: {}", coreos_url);

        // Parse the URL to extract address and port
        if let Ok(url) = url::Url::parse(&coreos_url) {
            let address = url.host_str().unwrap_or("localhost").to_string();
            let port = url.port().unwrap_or(8080);

            let coreos_service = service_registry::ServiceInfo::new(
                "coreos",
                "coreos-primary",
                address,
                port
            )
            .with_status(service_registry::ServiceStatus::Healthy)
            .with_metadata("auto_registered", "true");

            match registry.register(&coreos_service).await {
                Ok(lease_id) => {
                    tracing::info!("CoreOS registered successfully with lease ID: {}", lease_id);

                    // Wrap registry in Arc<Mutex> before spawning keepalive task
                    let registry = Arc::new(Mutex::new(registry));
                    let registry_for_keepalive = registry.clone();

                    tokio::spawn(async move {
                        loop {
                            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                            let mut reg = registry_for_keepalive.lock().await;
                            if let Err(e) = reg.keep_alive(lease_id).await {
                                tracing::error!("Failed to keep CoreOS lease alive: {}", e);
                                break;
                            }
                        }
                    });

                    // Build and run the app
                    let app = Router::new()
                        .route("/api/registry/register", post(api::register))
                        .route("/api/registry/deregister", post(api::deregister))
                        .route("/api/registry/services", get(api::list_services))
                        .route("/api/registry/services/{name}", get(api::get_services_by_name))
                        .route("/api/registry/keepalive", post(api::keep_alive))
                        .route("/health", get(|| async { "OK" }))
                        .with_state(registry)
                        .layer(TraceLayer::new_for_http());

                    let listener = tokio::net::TcpListener::bind("0.0.0.0:3003")
                        .await
                        .unwrap();

                    tracing::info!("Service registry listening on {}", listener.local_addr().unwrap());

                    axum::serve(listener, app).await.unwrap();
                    return;
                }
                Err(e) => {
                    tracing::error!("Failed to register CoreOS: {}", e);
                }
            }
        } else {
            tracing::error!("Invalid COREOS_URL format: {}", coreos_url);
        }
    }

    let registry = Arc::new(Mutex::new(registry));

    // Build the app
    let app = Router::new()
        .route("/api/registry/register", post(api::register))
        .route("/api/registry/deregister", post(api::deregister))
        .route("/api/registry/services", get(api::list_services))
        .route("/api/registry/services/{name}", get(api::get_services_by_name))
        .route("/api/registry/keepalive", post(api::keep_alive))
        .route("/health", get(|| async { "OK" }))
        .with_state(registry)
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3003")
        .await
        .unwrap();

    tracing::info!("Service registry listening on {}", listener.local_addr().unwrap());

    axum::serve(listener, app).await.unwrap();
}