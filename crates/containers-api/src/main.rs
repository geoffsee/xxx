use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{routing::get, Json, Router};
use podman_api::models::Namespace;
use podman_api::opts::ContainerCreateOpts;
use podman_api::opts::{ContainerListOpts, PullOpts, SocketNotifyMode, SystemdEnabled};
use podman_api::Podman;
use serde::Deserialize;
use serde_json::json;
use tokio_stream::StreamExt;
use tower_http::trace::TraceLayer;

async fn health() -> &'static str {
    "Ok"
}

async fn list_containers() -> impl IntoResponse {

    let podman_url = std::env::var("PODMAN_URL").unwrap_or("http://coreos:8085".to_string());

    let podman =  Podman::new(podman_url).unwrap();

    // List all containers including stopped ones
    let opts = ContainerListOpts::builder().all(true).build();
    let containers = podman.containers().list(&opts).await.unwrap();

    let container_strings = containers.iter().map(|container| {
        container.names.clone()
    });
    Json(container_strings.collect::<Vec<_>>())
}


#[derive(Deserialize)]
struct CreateContainerRequest {
    image: String,
    command: Option<Vec<String>>,
}

async fn create_container(Json(payload): Json<CreateContainerRequest>) -> impl IntoResponse {
    let podman_url = std::env::var("PODMAN_URL").unwrap_or("http://coreos:8085".to_string());
    let podman = Podman::new(podman_url).unwrap();

    // Build container creation options with host namespaces to avoid nested container issues
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

    // Before creating container, always try to pull the image
    println!("Pulling image '{}'...", payload.image);
    let pull_opts = PullOpts::builder().reference(&payload.image).build();
    let images = podman.images();
    let mut stream = images.pull(&pull_opts);

    // Process the stream to complete the pull operation
    while let Some(result) = stream.next().await {
        match result {
            Ok(info) => {
                println!("Pull progress: {:?}", info);
                // Check if the report contains an error
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

    // Create container
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

    // Get container ID from the response
    let id = created.id;

    // Start the container using its ID
    if let Err(e) = podman.containers().get(&id).start(None).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Container created but failed to start: {}", e),
        )
            .into_response();
    }

    println!("Container created and started successfully '{}'", id);
    // Return JSON success response
    (
        StatusCode::OK,
        Json(json!({
            "id": id,
            "message": "Container created and started successfully"
        })),
    )
        .into_response()
}

#[tokio::main]
async fn main() {

    dotenv::dotenv().ok();

    tracing_subscriber::fmt::init();

    let app = Router::new()
        .route("/healthz", get(health))
        .route("/api/containers/list", get(list_containers))
        .route("/api/containers/create", axum::routing::post(create_container))
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Server listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_hello_world() {
        let app = Router::new().route("/hello", get(health));

        let response = app
            .oneshot(Request::builder().uri("/hello").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(&body[..], b"Hello, World!");
    }
}

