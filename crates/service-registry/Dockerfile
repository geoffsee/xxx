# Build stage
FROM rust:1.90.0 as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/app

# Copy workspace manifests
COPY Cargo.toml Cargo.lock ./

# Copy all crates
COPY crates ./crates

# Build the application
# Use BuildKit cache mounts for Cargo registry and target dirs
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/src/app/target \
    cargo build --release -p service-registry && \
    cp /usr/src/app/target/release/service-registry /tmp/service-registry

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary from builder
COPY --from=builder /tmp/service-registry /app/service-registry

# Expose the port
EXPOSE 3003

# Run the application
CMD ["/app/service-registry"]