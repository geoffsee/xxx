use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServiceStatus {
    Healthy,
    Unhealthy,
    Starting,
    Stopping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub name: String,
    pub id: String,
    pub address: String,
    pub port: u16,
    pub status: ServiceStatus,
    pub metadata: HashMap<String, String>,
    pub version: String,
}

impl ServiceInfo {
    pub fn new(name: impl Into<String>, id: impl Into<String>, address: impl Into<String>, port: u16) -> Self {
        Self {
            name: name.into(),
            id: id.into(),
            address: address.into(),
            port,
            status: ServiceStatus::Starting,
            metadata: HashMap::new(),
            version: "0.1.0".to_string(),
        }
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    pub fn with_status(mut self, status: ServiceStatus) -> Self {
        self.status = status;
        self
    }

    pub fn service_key(&self) -> String {
        format!("/services/{}/{}", self.name, self.id)
    }
}