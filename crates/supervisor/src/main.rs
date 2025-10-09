use axum::{extract::State, response::IntoResponse, routing::get, Json, Router};
use serde::Serialize;
use service_registry::register_service;
use service_registry::ServiceInfo;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::trace::TraceLayer;

#[derive(Clone)]
struct AppState {
    registry_url: String,
    client: reqwest::Client,
}

#[derive(Serialize)]
struct HealthSummary {
    services: Vec<ServiceStatus>,
}

#[derive(Serialize)]
struct ServiceStatus {
    name: String,
    id: String,
    address: String,
    port: u16,
    registered: bool,
    http_health: Option<bool>,
    health_endpoint: Option<String>,
    notes: Option<String>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // Register the supervisor service for discovery/consistency
    let (service, _lease) = register_service!("supervisor", "supervisor", 3000).await;
    tracing::info!("Service registered: {} ({})", service.name, service.id);

    let registry_url = std::env::var("SERVICE_REGISTRY_URL")
        .unwrap_or_else(|_| "http://service-registry:3003".to_string());

    // Accept invalid certs so we can probe self-signed TLS services like repl-api
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .expect("failed building HTTP client");

    let state = AppState { registry_url, client };

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/supervisor/status", get(status))
        .with_state(state)
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    tracing::info!("supervisor listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

async fn health() -> &'static str {
    "OK"
}

async fn status(State(state): State<AppState>) -> impl IntoResponse {
    let url = format!("{}/api/registry/services", state.registry_url);
    let services: Vec<ServiceInfo> = match state.client.get(&url).send().await {
        Ok(resp) => match resp.json::<Vec<ServiceInfo>>().await {
            Ok(svcs) => svcs,
            Err(e) => {
                tracing::warn!("Failed to parse services from registry: {}", e);
                Vec::new()
            }
        },
        Err(e) => {
            tracing::warn!("Failed to query service registry: {}", e);
            Vec::new()
        }
    };

    // Build a quick map of probe functions by service name
    let mut probes: HashMap<&str, fn(&reqwest::Client, &ServiceInfo) -> ProbeTarget> = HashMap::new();
    probes.insert("container-api", |client, svc| ProbeTarget {
        url: format!("http://{}:{}/healthz", svc.address, svc.port),
        client: client.clone(),
        notes: None,
    });
    probes.insert("repl-api", |client, svc| ProbeTarget {
        // repl-api uses self-signed TLS in this repo
        url: format!("https://{}:{}/api/repl/languages", svc.address, svc.port),
        client: client.clone(),
        notes: Some("probed languages endpoint over self-signed TLS".to_string()),
    });
    // We skip HTTP probing for coreos & unknown services by default

    let mut statuses = Vec::new();
    for svc in services.iter() {
        let probe_entry = probes.get(svc.name.as_str());
        let (http_health, endpoint, notes) = if let Some(factory) = probe_entry {
            let target = factory(&state.client, svc);
            let ok = probe_http(&target).await;
            (Some(ok), Some(target.url), target.notes)
        } else {
            (None, None, Some("no health check configured".to_string()))
        };

        statuses.push(ServiceStatus {
            name: svc.name.clone(),
            id: svc.id.clone(),
            address: svc.address.clone(),
            port: svc.port,
            registered: true,
            http_health,
            health_endpoint: endpoint,
            notes,
        });
    }

    Json(HealthSummary { services: statuses })
}

struct ProbeTarget {
    url: String,
    client: reqwest::Client,
    notes: Option<String>,
}

async fn probe_http(target: &ProbeTarget) -> bool {
    match target.client.get(&target.url).send().await {
        Ok(resp) => resp.status().is_success(),
        Err(e) => {
            tracing::debug!("Probe failed for {}: {}", target.url, e);
            false
        }
    }
}

