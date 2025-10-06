use thiserror::Error;

#[derive(Error, Debug)]
pub enum RegistryError {
    #[error("etcd error: {0}")]
    EtcdError(#[from] etcd_client::Error),

    #[error("serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("service not found: {0}")]
    ServiceNotFound(String),

    #[error("invalid service data: {0}")]
    InvalidServiceData(String),

    #[error("connection error: {0}")]
    ConnectionError(String),
}

pub type Result<T> = std::result::Result<T, RegistryError>;