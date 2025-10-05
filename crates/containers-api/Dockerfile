# Build stage
FROM rust:1.90.0 as builder

WORKDIR /usr/src/app

# Copy workspace manifests
COPY Cargo.toml Cargo.lock ./

# Copy all workspace members
COPY crates ./crates
COPY repl-api ./repl-api

# Build the application
RUN cargo build --release --manifest-path ./crates/containers-api/Cargo.toml

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary from builder
COPY --from=builder /usr/src/app/target/release/containers-api /app/containers-api

# Expose the port
EXPOSE 3000

# Run the application
CMD ["/app/containers-api"]