# Build stage
FROM rust:1.90.0 as builder

WORKDIR /usr/src/app

# Copy workspace manifests
COPY Cargo.toml Cargo.lock ./

# Copy all crates
COPY crates ./crates

# Build the application
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/src/app/target \
    cargo build --release -p supervisor && \
    cp /usr/src/app/target/release/supervisor /tmp/supervisor

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary from builder
COPY --from=builder /tmp/supervisor /app/supervisor

# Expose the port
EXPOSE 3000

# Run the application
CMD ["/app/supervisor"]

