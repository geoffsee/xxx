use service_registry::{ServiceRegistry, ServiceInfo, ServiceStatus};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Connect to etcd (make sure etcd is running on localhost:2379)
    let mut registry = ServiceRegistry::new(
        vec!["localhost:2379".to_string()],
        Some(10)
    ).await?;

    // Create a service
    let service = ServiceInfo::new(
        "example-service",
        "instance-1",
        "localhost",
        8080
    )
    .with_status(ServiceStatus::Healthy)
    .with_metadata("version", "1.0.0")
    .with_metadata("env", "development");

    println!("Registering service: {} ({})", service.name, service.id);

    // Register the service
    let lease_id = registry.register(&service).await?;
    println!("Service registered with lease ID: {}", lease_id);

    // Discover all instances of this service
    println!("\nDiscovering services...");
    let services = registry.get_services("example-service").await?;
    println!("Found {} instance(s):", services.len());
    for svc in &services {
        println!("  - {} at {}:{}", svc.id, svc.address, svc.port);
    }

    // Keep the service alive for 30 seconds
    println!("\nKeeping service alive for 30 seconds...");
    for i in 1..=6 {
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        registry.keep_alive(lease_id).await?;
        println!("  Keep-alive sent ({}/6)", i);
    }

    // Deregister the service
    println!("\nDeregistering service...");
    registry.deregister(&service).await?;
    println!("Service deregistered successfully");

    Ok(())
}