pub mod registry;
pub mod error;
pub mod service;
pub mod bootstrap;
pub mod api;

pub use registry::ServiceRegistry;
pub use error::RegistryError;
pub use service::{ServiceInfo, ServiceStatus};
pub use bootstrap::{bootstrap_service, get_service_endpoint};

// Re-export the macro
pub use service_registry_macros::register_service;