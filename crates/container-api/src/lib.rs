use axum::extract::Path;
use axum::Json;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::sse::{Event, Sse};
use futures_util::{Stream, TryStreamExt};
use podman_api::Podman;
use podman_api::models::Namespace;
use podman_api::opts::{ContainerCreateOpts, ContainerStopOpts, ContainerWaitOpts};
use podman_api::opts::{ContainerListOpts, PullOpts, SocketNotifyMode, SystemdEnabled};
use serde::Deserialize;
use serde_json::json;
use tokio_stream::StreamExt;
use std::convert::Infallible;

pub async fn health() -> &'static str {
    "Ok"
}

pub async fn list_containers() -> impl IntoResponse {
    let podman_url = match service_registry::bootstrap::get_service_endpoint("coreos").await {
        Some(url) => url,
        None => std::env::var("COREOS_URL").unwrap_or("http://coreos:8085".to_string()),
    };
    let podman = Podman::new(podman_url).unwrap();

    let opts = ContainerListOpts::builder().all(true).build();
    let containers = podman.containers().list(&opts).await.unwrap();

    let container_strings = containers.iter().map(|container| container.names.clone());
    Json(container_strings.collect::<Vec<_>>())
}

#[derive(Deserialize)]
pub struct CreateContainerRequest {
    pub image: String,
    pub command: Option<Vec<String>>,
}

pub async fn create_container(Json(payload): Json<CreateContainerRequest>) -> impl IntoResponse {
    let podman_url = match service_registry::bootstrap::get_service_endpoint("coreos").await {
        Some(url) => url,
        None => std::env::var("COREOS_URL").unwrap_or("http://coreos:8085".to_string()),
    };
    let podman = Podman::new(podman_url).unwrap();

    let opts = ContainerCreateOpts::builder()
        .image(&payload.image)
        .command(payload.command.unwrap_or_default())
        .net_namespace(Namespace {
            nsmode: Some("private".to_string()),
            value: None,
        })
        .pid_namespace(Namespace {
            nsmode: Some("private".to_string()),
            value: None,
        })
        .ipc_namespace(Namespace {
            nsmode: Some("private".to_string()),
            value: None,
        })
        .systemd(SystemdEnabled::False)
        .sdnotify_mode(SocketNotifyMode::Ignore)
        .build();

    println!("Pulling image '{}'...", payload.image);
    let pull_opts = PullOpts::builder().reference(&payload.image).build();
    let images = podman.images();
    let mut stream = images.pull(&pull_opts);

    while let Some(result) = stream.next().await {
        match result {
            Ok(info) => {
                println!("Pull progress: {:?}", info);
                if let Some(error_msg) = &info.error {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Failed to pull image '{}': {}", payload.image, error_msg),
                    )
                        .into_response();
                }
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to pull image '{}': {}", payload.image, e),
                )
                    .into_response();
            }
        }
    }
    println!("Successfully pulled image '{}'", payload.image);

    let created = match podman.containers().create(&opts).await {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create container: {}", e),
            )
                .into_response();
        }
    };

    let id = created.id;

    let container = podman.containers().get(&id);

    if let Err(e) = container.start(None).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Container created but failed to start: {}", e),
        )
            .into_response();
    }

    println!("Container '{}' started, waiting for completion...", id);

    // Wait for the container to finish
    if let Err(e) = container.wait(&ContainerWaitOpts::builder().build()).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Error waiting for container to finish: {}", e),
        )
            .into_response();
    }

    // Get container logs (stdout + stderr)
    let logs = match container.logs(
        &podman_api::opts::ContainerLogsOpts::builder()
            .stdout(true)
            .stderr(true)
            .build()
    ).try_collect::<Vec<_>>().await {
        Ok(chunks) => {
            chunks.iter()
                .map(|chunk| String::from_utf8_lossy(chunk.as_ref()))
                .collect::<String>()
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to get container logs: {}", e),
            )
                .into_response();
        }
    };

    // Clean up the container
    let _ = container.remove().await;

    println!("Container '{}' completed successfully", id);
    (
        StatusCode::OK,
        Json(json!({
            "id": id,
            "message": "Container executed successfully",
            "output": logs
        })),
    )
        .into_response()
}

pub async fn create_container_stream(
    Json(payload): Json<CreateContainerRequest>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = async_stream::stream! {
        let podman_url = match service_registry::bootstrap::get_service_endpoint("coreos").await {
            Some(url) => url,
            None => std::env::var("COREOS_URL").unwrap_or("http://coreos:8085".to_string()),
        };

        let podman = match Podman::new(podman_url) {
            Ok(p) => p,
            Err(e) => {
                yield Ok(Event::default().data(format!("ERROR: Failed to connect to Podman: {}", e)));
                return;
            }
        };

        let opts = ContainerCreateOpts::builder()
            .image(&payload.image)
            .command(payload.command.unwrap_or_default())
            .net_namespace(Namespace {
                nsmode: Some("private".to_string()),
                value: None,
            })
            .pid_namespace(Namespace {
                nsmode: Some("private".to_string()),
                value: None,
            })
            .ipc_namespace(Namespace {
                nsmode: Some("private".to_string()),
                value: None,
            })
            .systemd(SystemdEnabled::False)
            .sdnotify_mode(SocketNotifyMode::Ignore)
            .build();

        // Pull image
        let pull_opts = PullOpts::builder().reference(&payload.image).build();
        let images = podman.images();
        let mut pull_stream = images.pull(&pull_opts);

        while let Some(result) = pull_stream.next().await {
            match result {
                Ok(info) => {
                    if let Some(error_msg) = &info.error {
                        yield Ok(Event::default().data(format!("ERROR: Failed to pull image '{}': {}", payload.image, error_msg)));
                        return;
                    }
                }
                Err(e) => {
                    yield Ok(Event::default().data(format!("ERROR: Failed to pull image '{}': {}", payload.image, e)));
                    return;
                }
            }
        }

        // Create container
        let created = match podman.containers().create(&opts).await {
            Ok(c) => c,
            Err(e) => {
                yield Ok(Event::default().data(format!("ERROR: Failed to create container: {}", e)));
                return;
            }
        };

        let id = created.id.clone();
        let container = podman.containers().get(&id);

        // Attach to container to get output stream
        use podman_api::opts::ContainerAttachOpts;
        let attach_opts = ContainerAttachOpts::builder()
            .stdout(true)
            .stderr(true)
            .build();

        let mut attach_stream = match container.attach(&attach_opts).await {
            Ok(stream) => stream,
            Err(e) => {
                yield Ok(Event::default().data(format!("ERROR: Failed to attach to container: {}", e)));
                return;
            }
        };

        // Start container after attaching
        if let Err(e) = container.start(None).await {
            yield Ok(Event::default().data(format!("ERROR: Container failed to start: {}", e)));
            return;
        }

        // Stream output as it comes in
        while let Some(chunk_result) = attach_stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    let output = String::from_utf8_lossy(&chunk);
                    if !output.is_empty() {
                        yield Ok(Event::default().data(output.to_string()));
                    }
                }
                Err(e) => {
                    yield Ok(Event::default().data(format!("ERROR: Failed to read output: {}", e)));
                    break;
                }
            }
        }

        // Wait for container to finish
        let _ = container.wait(&ContainerWaitOpts::builder().build()).await;

        // Clean up
        let _ = container.remove().await;

        yield Ok(Event::default().event("done").data("Container execution completed"));
    };

    Sse::new(stream)
}

pub async fn remove_container(Path(id): Path<String>) -> impl IntoResponse {
    let podman_url = match service_registry::bootstrap::get_service_endpoint("coreos").await {
        Some(url) => url,
        None => std::env::var("COREOS_URL").unwrap_or("http://coreos:8085".to_string()),
    };
    let podman = match Podman::new(podman_url) {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to connect to Podman service: {}", e),
            )
                .into_response();
        }
    };

    let container = podman.containers().get(&id);

    // Attempt to stop the container first
    println!("Stopping container '{}'...", id);
    match container.stop(&ContainerStopOpts::builder().build()).await {
        Ok(_) => println!("Container '{}' stopped successfully", id),
        Err(e) => println!(
            "Warning: could not stop container '{}': {} (continuing with removal)",
            id, e
        ),
    }

    // Attempt to remove the container
    println!("Removing container '{}'...", id);
    match container.remove().await {
        Ok(_) => {
            println!("Container '{}' removed successfully", id);
            (
                StatusCode::OK,
                Json(json!({
                    "id": id,
                    "message": "Container removed successfully"
                })),
            )
                .into_response()
        }
        Err(e) => {
            println!("Failed to remove container '{}': {}", id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to remove container '{}': {}", id, e),
            )
                .into_response()
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::Router;
    use axum::routing::get;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_health() {
        let result = health().await;
        assert_eq!(result, "Ok");
    }

    #[tokio::test]
    async fn test_health_endpoint() {
        let app = Router::new().route("/health", get(health));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(&body[..], b"Ok");
    }

    #[test]
    fn test_create_container_request_deserialization() {
        let json = r#"{"image":"python:3.11","command":["python","-c","print('hello')"]}"#;
        let request: CreateContainerRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.image, "python:3.11");
        assert_eq!(request.command, Some(vec!["python".to_string(), "-c".to_string(), "print('hello')".to_string()]));
    }

    #[test]
    fn test_create_container_request_deserialization_no_command() {
        let json = r#"{"image":"python:3.11"}"#;
        let request: CreateContainerRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.image, "python:3.11");
        assert_eq!(request.command, None);
    }
}
