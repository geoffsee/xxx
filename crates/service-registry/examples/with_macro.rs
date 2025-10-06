use service_registry::register_service;

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Register service using the macro - automatically handles:
    // - Reading ETCD_ENDPOINTS from environment
    // - Generating unique service ID
    // - Starting keep-alive background task
    println!("Registering service using macro...");
    let (service, lease_id) = register_service!("macro-service", "localhost", 9090).await;

    println!("Service registered!");
    println!("  Name: {}", service.name);
    println!("  ID: {}", service.id);
    println!("  Address: {}:{}", service.address, service.port);
    println!("  Lease ID: {}", lease_id);

    // Your application logic here
    println!("\nService running... (Press Ctrl+C to exit)");

    // Keep the application running
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
        println!("Service still running...");
    }
}