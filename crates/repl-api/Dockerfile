# Build stage
FROM rust:1.90.0 as builder

WORKDIR /usr/src/app

# Copy workspace manifests
COPY Cargo.toml Cargo.lock ./

# Copy all workspace members
COPY crates ./crates

# Build the application
RUN cargo build --release --package repl-api

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary from builder
COPY --from=builder /usr/src/app/target/release/repl-api /app/repl-api

# Expose the port
EXPOSE 3000

# Run the application
CMD ["/app/repl-api"]