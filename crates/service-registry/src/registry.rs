use crate::error::{RegistryError, Result};
use crate::service::ServiceInfo;
use etcd_client::{Client, GetOptions, PutOptions};
use tracing::{debug, info, warn};

/// ServiceRegistry provides service discovery and registration using etcd as the backend
pub struct ServiceRegistry {
    client: Client,
    lease_ttl: i64,
}

impl ServiceRegistry {
    /// Create a new ServiceRegistry instance
    ///
    /// # Arguments
    /// * `endpoints` - List of etcd endpoints (e.g., ["localhost:2379"])
    /// * `lease_ttl` - Time-to-live for service registrations in seconds (default: 10)
    pub async fn new(endpoints: Vec<String>, lease_ttl: Option<i64>) -> Result<Self> {
        info!("Connecting to etcd at endpoints: {:?}", endpoints);

        let client = Client::connect(endpoints, None)
            .await
            .map_err(|e| RegistryError::ConnectionError(e.to_string()))?;

        Ok(Self {
            client,
            lease_ttl: lease_ttl.unwrap_or(10),
        })
    }

    /// Register a service with the registry
    ///
    /// This creates a lease and associates the service with it for automatic cleanup
    pub async fn register(&mut self, service: &ServiceInfo) -> Result<i64> {
        let key = service.service_key();
        let value = serde_json::to_string(service)?;

        debug!("Registering service at key: {}", key);

        // Create a lease
        let lease = self.client.lease_grant(self.lease_ttl, None).await?;
        let lease_id = lease.id();

        info!(
            "Created lease {} with TTL {} seconds for service {}",
            lease_id, self.lease_ttl, service.name
        );

        // Put the service info with the lease
        let put_options = PutOptions::new().with_lease(lease_id);
        self.client
            .put(key.clone(), value, Some(put_options))
            .await?;

        info!("Service {} registered successfully at {}", service.name, key);

        Ok(lease_id)
    }

    /// Keep a service registration alive by refreshing its lease
    pub async fn keep_alive(&mut self, lease_id: i64) -> Result<()> {
        debug!("Keeping lease {} alive", lease_id);

        let (mut keeper, mut stream) = self.client.lease_keep_alive(lease_id).await?;

        // Send initial keep alive
        keeper.keep_alive().await?;

        // Check response
        if let Some(resp) = stream.message().await? {
            debug!("Lease {} kept alive, TTL: {}", resp.id(), resp.ttl());
        }

        Ok(())
    }

    /// Deregister a service from the registry
    pub async fn deregister(&mut self, service: &ServiceInfo) -> Result<()> {
        let key = service.service_key();

        info!("Deregistering service at key: {}", key);

        self.client.delete(key.clone(), None).await?;

        info!("Service {} deregistered successfully", service.name);

        Ok(())
    }

    /// Get a specific service by name and id
    pub async fn get_service(&mut self, service_name: &str, service_id: &str) -> Result<ServiceInfo> {
        let key = format!("/services/{}/{}", service_name, service_id);

        debug!("Getting service at key: {}", key);

        let resp = self.client.get(key.clone(), None).await?;

        if let Some(kv) = resp.kvs().first() {
            let service: ServiceInfo = serde_json::from_slice(kv.value())?;
            Ok(service)
        } else {
            Err(RegistryError::ServiceNotFound(key))
        }
    }

    /// Get all instances of a service by name
    pub async fn get_services(&mut self, service_name: &str) -> Result<Vec<ServiceInfo>> {
        let key = format!("/services/{}/", service_name);

        debug!("Getting all services with prefix: {}", key);

        let get_options = GetOptions::new().with_prefix();
        let resp = self.client.get(key, Some(get_options)).await?;

        let mut services = Vec::new();
        for kv in resp.kvs() {
            match serde_json::from_slice(kv.value()) {
                Ok(service) => services.push(service),
                Err(e) => {
                    warn!("Failed to deserialize service: {}", e);
                    continue;
                }
            }
        }

        info!("Found {} instances of service {}", services.len(), service_name);

        Ok(services)
    }

    /// Get all registered services
    pub async fn get_all_services(&mut self) -> Result<Vec<ServiceInfo>> {
        let key = "/services/";

        debug!("Getting all services");

        let get_options = GetOptions::new().with_prefix();
        let resp = self.client.get(key, Some(get_options)).await?;

        let mut services = Vec::new();
        for kv in resp.kvs() {
            match serde_json::from_slice(kv.value()) {
                Ok(service) => services.push(service),
                Err(e) => {
                    warn!("Failed to deserialize service: {}", e);
                    continue;
                }
            }
        }

        info!("Found {} total registered services", services.len());

        Ok(services)
    }

    /// Watch for changes to a specific service
    pub async fn watch_service(&mut self, service_name: &str) -> Result<()> {
        let key = format!("/services/{}/", service_name);

        info!("Watching service: {}", service_name);

        let (_watcher, mut stream) = self.client.watch(key, None).await?;

        while let Some(resp) = stream.message().await? {
            for event in resp.events() {
                debug!("Event: {:?}", event);
            }
        }

        Ok(())
    }
}