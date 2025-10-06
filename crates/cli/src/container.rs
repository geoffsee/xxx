use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
pub struct CreateContainerRequest {
    pub image: String,
    pub command: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct CreateContainerResponse {
    pub id: String,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct RemoveContainerResponse {
    pub id: String,
    pub message: String,
}

pub struct ContainerClient {
    base_url: String,
    client: reqwest::Client,
}

impl ContainerClient {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            client: reqwest::Client::new(),
        }
    }

    pub async fn list_containers(&self) -> Result<Vec<Vec<String>>> {
        let url = format!("{}/api/containers/list", self.base_url);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send list containers request")?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Failed to list containers: {}", error_text);
        }

        let containers: Vec<Vec<String>> = response
            .json()
            .await
            .context("Failed to parse list containers response")?;

        Ok(containers)
    }

    pub async fn create_container(
        &self,
        image: String,
        command: Option<Vec<String>>,
    ) -> Result<CreateContainerResponse> {
        let url = format!("{}/api/containers/create", self.base_url);
        let request = CreateContainerRequest { image, command };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to send create container request")?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Failed to create container: {}", error_text);
        }

        let container_response: CreateContainerResponse = response
            .json()
            .await
            .context("Failed to parse create container response")?;

        Ok(container_response)
    }

    pub async fn remove_container(&self, id: String) -> Result<RemoveContainerResponse> {
        let url = format!("{}/api/containers/{}", self.base_url, id);

        let response = self
            .client
            .delete(&url)
            .send()
            .await
            .context("Failed to send remove container request")?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Failed to remove container: {}", error_text);
        }

        let remove_response: RemoveContainerResponse = response
            .json()
            .await
            .context("Failed to parse remove container response")?;

        Ok(remove_response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_container_request_serialization() {
        let request = CreateContainerRequest {
            image: "python:3.11".to_string(),
            command: Some(vec!["python".to_string(), "-c".to_string(), "print('hello')".to_string()]),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("python:3.11"));
        assert!(json.contains("python"));
        assert!(json.contains("print('hello')"));
    }

    #[test]
    fn test_create_container_request_serialization_no_command() {
        let request = CreateContainerRequest {
            image: "nginx:latest".to_string(),
            command: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("nginx:latest"));
        assert!(json.contains("null"));
    }

    #[test]
    fn test_create_container_response_deserialization() {
        let json = r#"{"id":"abc123","message":"Container created successfully"}"#;
        let response: CreateContainerResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.id, "abc123");
        assert_eq!(response.message, "Container created successfully");
    }

    #[test]
    fn test_container_client_creation() {
        let client = ContainerClient::new("http://localhost:3000".to_string());
        assert_eq!(client.base_url, "http://localhost:3000");
    }

    #[test]
    fn test_container_client_with_custom_url() {
        let client = ContainerClient::new("http://example.com:8080".to_string());
        assert_eq!(client.base_url, "http://example.com:8080");
    }

    #[test]
    fn test_create_container_request_with_empty_command() {
        let request = CreateContainerRequest {
            image: "redis:alpine".to_string(),
            command: Some(vec![]),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("redis:alpine"));
        assert!(json.contains("[]"));
    }

    #[test]
    fn test_create_container_request_with_single_command() {
        let request = CreateContainerRequest {
            image: "alpine:latest".to_string(),
            command: Some(vec!["sh".to_string()]),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("alpine:latest"));
        assert!(json.contains("sh"));
    }

    #[test]
    fn test_create_container_response_deserialization_with_long_id() {
        let json = r#"{"id":"e4d2f1a3b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9e0","message":"Success"}"#;
        let response: CreateContainerResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.id, "e4d2f1a3b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9e0");
        assert_eq!(response.message, "Success");
    }

    #[test]
    fn test_list_containers_response_deserialization() {
        let json = r#"[["container1", "alias1"], ["container2", "alias2"]]"#;
        let containers: Vec<Vec<String>> = serde_json::from_str(json).unwrap();
        assert_eq!(containers.len(), 2);
        assert_eq!(containers[0], vec!["container1", "alias1"]);
        assert_eq!(containers[1], vec!["container2", "alias2"]);
    }

    #[test]
    fn test_empty_list_containers_response_deserialization() {
        let json = r#"[]"#;
        let containers: Vec<Vec<String>> = serde_json::from_str(json).unwrap();
        assert!(containers.is_empty());
    }
}