use axum::{routing::get, routing::post, Router};
use tower_http::trace::TraceLayer;

// Import handlers from other crates
use container_api::{create_container, health, list_containers};
use repl_api::{execute_repl, list_languages};

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt::init();

    let app = Router::new()
        // Health check
        .route("/healthz", get(health))

        // Container API routes
        .route("/api/containers/list", get(list_containers))
        .route("/api/containers/create", post(create_container))

        // REPL API routes
        .route("/api/repl/execute", post(execute_repl))
        .route("/api/repl/languages", get(list_languages))

        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Server listening on {}", listener.local_addr().unwrap());
    println!("Available endpoints:");
    println!("  GET  /healthz");
    println!("  GET  /api/containers/list");
    println!("  POST /api/containers/create");
    println!("  POST /api/repl/execute");
    println!("  GET  /api/repl/languages");

    axum::serve(listener, app).await.unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_health_check() {
        let app = Router::new().route("/healthz", get(health));

        let response = app
            .oneshot(Request::builder().uri("/healthz").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(&body[..], b"Ok");
    }

    #[tokio::test]
    async fn test_list_languages() {
        let app = Router::new().route("/api/repl/languages", get(list_languages));

        let response = app
            .oneshot(Request::builder().uri("/api/repl/languages").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}