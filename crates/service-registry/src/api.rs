use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{ServiceRegistry, ServiceInfo};

type AppState = Arc<Mutex<ServiceRegistry>>;

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterRequest {
    pub service: ServiceInfo,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterResponse {
    pub lease_id: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct KeepAliveRequest {
    pub lease_id: i64,
}

pub async fn register(
    State(registry): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<RegisterResponse>, StatusCode> {
    let mut registry = registry.lock().await;

    match registry.register(&req.service).await {
        Ok(lease_id) => {
            tracing::info!("Registered service: {} with lease {}", req.service.name, lease_id);
            Ok(Json(RegisterResponse { lease_id }))
        }
        Err(e) => {
            tracing::error!("Failed to register service: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn deregister(
    State(registry): State<AppState>,
    Json(service): Json<ServiceInfo>,
) -> Result<StatusCode, StatusCode> {
    let mut registry = registry.lock().await;

    match registry.deregister(&service).await {
        Ok(_) => {
            tracing::info!("Deregistered service: {}", service.name);
            Ok(StatusCode::OK)
        }
        Err(e) => {
            tracing::error!("Failed to deregister service: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn list_services(
    State(registry): State<AppState>,
) -> Result<Json<Vec<ServiceInfo>>, StatusCode> {
    let mut registry = registry.lock().await;

    match registry.get_all_services().await {
        Ok(services) => Ok(Json(services)),
        Err(e) => {
            tracing::error!("Failed to list services: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn get_services_by_name(
    State(registry): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<Vec<ServiceInfo>>, StatusCode> {
    let mut registry = registry.lock().await;

    match registry.get_services(&name).await {
        Ok(services) => Ok(Json(services)),
        Err(e) => {
            tracing::error!("Failed to get services by name {}: {}", name, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn keep_alive(
    State(registry): State<AppState>,
    Json(req): Json<KeepAliveRequest>,
) -> Result<StatusCode, StatusCode> {
    let mut registry = registry.lock().await;

    match registry.keep_alive(req.lease_id).await {
        Ok(_) => Ok(StatusCode::OK),
        Err(e) => {
            tracing::error!("Failed to keep alive lease {}: {}", req.lease_id, e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}