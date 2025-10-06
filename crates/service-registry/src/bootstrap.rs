use crate::{ServiceInfo, ServiceStatus};
use std::env;
use tracing::{debug, warn};

/// Bootstrap a service with automatic registration via service-registry HTTP API
///
/// This function:
/// - Reads SERVICE_REGISTRY_URL from environment (defaults to http://service-registry:3003)
/// - Generates a unique service ID from hostname and PID
/// - Registers the service via HTTP
/// - Spawns a background task to keep the lease alive
///
/// Returns the ServiceInfo and lease_id
pub async fn bootstrap_service(
    service_name: impl Into<String>,
    address: impl Into<String>,
    port: u16,
) -> (ServiceInfo, i64) {
    // Get service registry URL from environment
    let registry_url = env::var("SERVICE_REGISTRY_URL")
        .unwrap_or_else(|_| "http://service-registry:3003".to_string());

    // Create service ID from hostname and PID
    let hostname = hostname::get()
        .unwrap_or_else(|_| std::ffi::OsString::from("unknown"))
        .to_string_lossy()
        .to_string();
    let pid = std::process::id();
    let service_id = format!("{}-{}", hostname, pid);

    // Create service info
    let service = ServiceInfo::new(
        service_name,
        service_id,
        address,
        port
    )
    .with_status(ServiceStatus::Healthy);

    // Register service via HTTP with retry logic
    let client = reqwest::Client::new();

    #[derive(serde::Deserialize)]
    struct RegisterResponse {
        lease_id: i64,
    }

    let mut attempts = 0;
    let max_attempts = 30;
    let lease_id = loop {
        attempts += 1;

        match client
            .post(format!("{}/api/registry/register", registry_url))
            .json(&serde_json::json!({ "service": service }))
            .send()
            .await
        {
            Ok(response) => {
                match response.json::<RegisterResponse>().await {
                    Ok(register_response) => {
                        tracing::info!("Service registered with lease ID: {}", register_response.lease_id);
                        break register_response.lease_id;
                    }
                    Err(e) => {
                        tracing::warn!("Failed to parse registration response: {}", e);
                        if attempts >= max_attempts {
                            panic!("Failed to register service after {} attempts", max_attempts);
                        }
                    }
                }
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to register service (attempt {}/{}): {}",
                    attempts, max_attempts, e
                );
                if attempts >= max_attempts {
                    panic!("Failed to register service after {} attempts: {}", max_attempts, e);
                }
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            }
        }
    };

    // Keep-alive task
    let registry_url_clone = registry_url.clone();
    tokio::spawn(async move {
        let client = reqwest::Client::new();
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            let result = client
                .post(format!("{}/api/registry/keepalive", registry_url_clone))
                .json(&serde_json::json!({ "lease_id": lease_id }))
                .send()
                .await;

            if let Err(e) = result {
                tracing::error!("Failed to keep lease alive: {}", e);
                break;
            }
        }
    });

    (service, lease_id)
}

/// Get the endpoint URL for a service by name
///
/// This function queries the service registry HTTP API to find an available
/// instance of the requested service and returns its endpoint URL.
///
/// Returns None if the service is not found or if there's an error.
pub async fn get_service_endpoint(service_name: &str) -> Option<String> {
    let registry_url = env::var("SERVICE_REGISTRY_URL")
        .unwrap_or_else(|_| "http://service-registry:3003".to_string());

    debug!("Looking up service: {}", service_name);

    let client = reqwest::Client::new();

    match client
        .get(format!("{}/api/registry/services/{}", registry_url, service_name))
        .send()
        .await
    {
        Ok(response) => {
            if response.status().is_success() {
                match response.json::<Vec<ServiceInfo>>().await {
                    Ok(services) => {
                        if let Some(service) = services.first() {
                            let endpoint = format!("http://{}:{}", service.address, service.port);
                            debug!("Found service {} at {}", service_name, endpoint);
                            Some(endpoint)
                        } else {
                            warn!("No instances found for service: {}", service_name);
                            None
                        }
                    }
                    Err(e) => {
                        warn!("Failed to parse services response: {}", e);
                        None
                    }
                }
            } else {
                warn!("Service registry returned error for {}: {}", service_name, response.status());
                None
            }
        }
        Err(e) => {
            warn!("Failed to query service registry for {}: {}", service_name, e);
            None
        }
    }
}