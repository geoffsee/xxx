use axum::{routing::get, Router};
use tower_http::trace::TraceLayer;
use container_api::{health, list_containers, create_container};

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

