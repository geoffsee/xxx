use axum::Json;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use podman_api::Podman;
use podman_api::models::Namespace;
use podman_api::opts::ContainerCreateOpts;
use podman_api::opts::{ContainerListOpts, PullOpts, SocketNotifyMode, SystemdEnabled};
use serde::Deserialize;
use serde_json::json;
use tokio_stream::StreamExt;

pub async fn health() -> &'static str {
    "Ok"
}

pub async fn list_containers() -> impl IntoResponse {
    let podman_url = std::env::var("PODMAN_URL").unwrap_or("http://coreos:8085".to_string());
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
    let podman_url = std::env::var("PODMAN_URL").unwrap_or("http://coreos:8085".to_string());
    let podman = Podman::new(podman_url).unwrap();

    let opts = ContainerCreateOpts::builder()
        .image(&payload.image)
        .command(payload.command.unwrap_or_default())
        .net_namespace(Namespace {
            nsmode: Some("host".to_string()),
            value: None,
        })
        .pid_namespace(Namespace {
            nsmode: Some("host".to_string()),
            value: None,
        })
        .ipc_namespace(Namespace {
            nsmode: Some("host".to_string()),
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

    if let Err(e) = podman.containers().get(&id).start(None).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Container created but failed to start: {}", e),
        )
            .into_response();
    }

    println!("Container created and started successfully '{}'", id);
    (
        StatusCode::OK,
        Json(json!({
            "id": id,
            "message": "Container created and started successfully"
        })),
    )
        .into_response()
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
