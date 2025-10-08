mod tls;

use axum::{
    routing::{get, post},
    Router,
};
use service_registry::register_service;
use std::net::SocketAddr;
use std::path::PathBuf;
use axum_server::tls_rustls::RustlsConfig;
use crate::tls::make_cert;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("repl-api server starting...");


    let app = Router::new()
        .route("/api/repl/execute", post(repl_api::execute_repl))
        .route("/api/repl/execute/stream", post(repl_api::execute_repl_stream))
        .route("/api/repl/languages", get(repl_api::list_languages));

    // Generate a self-signed cert (via your tls module)
    let (cert_pem, key_pem) = make_cert();

    // Use the in-memory PEM data directly
    let tls_config = RustlsConfig::from_pem(cert_pem.into_bytes(), key_pem.into_bytes())
        .await
        .expect("failed to build RustlsConfig from cert data");

    // Bind HTTPS on port 3001
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("repl-api listening securely on https://{}", addr);
    let (service, _lease_id) = register_service!("repl-api", "repl-api", 3000).await;
    tracing::info!("Service registered: {} ({})", service.name, service.id);
    axum_server::bind_rustls(addr, tls_config)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
