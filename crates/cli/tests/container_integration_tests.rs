use mockito::{Server, ServerGuard};

// Re-export the container module types for testing
mod container {
    pub use cli::container::*;
}

use container::{ContainerClient, CreateContainerResponse};

async fn setup_mock_server() -> ServerGuard {
    Server::new_async().await
}

#[tokio::test]
async fn test_list_containers_success() {
    let mut server = setup_mock_server().await;

    let mock = server
        .mock("GET", "/api/containers/list")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"[["container1", "alias1"], ["container2"]]"#)
        .create_async()
        .await;

    let client = ContainerClient::new(server.url());
    let result = client.list_containers().await;

    mock.assert_async().await;
    assert!(result.is_ok());
    let containers = result.unwrap();
    assert_eq!(containers.len(), 2);
    assert_eq!(containers[0], vec!["container1", "alias1"]);
    assert_eq!(containers[1], vec!["container2"]);
}

#[tokio::test]
async fn test_list_containers_empty() {
    let mut server = setup_mock_server().await;

    let mock = server
        .mock("GET", "/api/containers/list")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("[]")
        .create_async()
        .await;

    let client = ContainerClient::new(server.url());
    let result = client.list_containers().await;

    mock.assert_async().await;
    assert!(result.is_ok());
    let containers = result.unwrap();
    assert!(containers.is_empty());
}

#[tokio::test]
async fn test_list_containers_error() {
    let mut server = setup_mock_server().await;

    let mock = server
        .mock("GET", "/api/containers/list")
        .with_status(500)
        .with_body("Internal server error")
        .create_async()
        .await;

    let client = ContainerClient::new(server.url());
    let result = client.list_containers().await;

    mock.assert_async().await;
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("Failed to list containers"));
}

#[tokio::test]
async fn test_create_container_success() {
    let mut server = setup_mock_server().await;

    let mock = server
        .mock("POST", "/api/containers/create")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"id":"abc123","message":"Container created successfully"}"#)
        .create_async()
        .await;

    let client = ContainerClient::new(server.url());
    let result = client
        .create_container(
            "python:3.11".to_string(),
            Some(vec!["python".to_string(), "-c".to_string(), "print('hello')".to_string()]),
        )
        .await;

    mock.assert_async().await;
    assert!(result.is_ok());
    let response: CreateContainerResponse = result.unwrap();
    assert_eq!(response.id, "abc123");
    assert_eq!(response.message, "Container created successfully");
}

#[tokio::test]
async fn test_create_container_no_command() {
    let mut server = setup_mock_server().await;

    let mock = server
        .mock("POST", "/api/containers/create")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"id":"xyz789","message":"Container started"}"#)
        .create_async()
        .await;

    let client = ContainerClient::new(server.url());
    let result = client.create_container("nginx:latest".to_string(), None).await;

    mock.assert_async().await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.id, "xyz789");
    assert_eq!(response.message, "Container started");
}

#[tokio::test]
async fn test_create_container_error() {
    let mut server = setup_mock_server().await;

    let mock = server
        .mock("POST", "/api/containers/create")
        .with_status(500)
        .with_body("Failed to pull image")
        .create_async()
        .await;

    let client = ContainerClient::new(server.url());
    let result = client
        .create_container("invalid:image".to_string(), None)
        .await;

    mock.assert_async().await;
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("Failed to create container"));
}

#[tokio::test]
async fn test_create_container_with_empty_command() {
    let mut server = setup_mock_server().await;

    let mock = server
        .mock("POST", "/api/containers/create")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"id":"def456","message":"Success"}"#)
        .create_async()
        .await;

    let client = ContainerClient::new(server.url());
    let result = client
        .create_container("alpine:latest".to_string(), Some(vec![]))
        .await;

    mock.assert_async().await;
    assert!(result.is_ok());
}