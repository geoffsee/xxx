# xxx - REPL-as-a-Service Platform

xxx provides ephemeral, isolated execution environments for multiple programming languages through a scalable, container-orchestrated architecture.

---

## Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
- [Features](#features)
- [Prerequisites](#prerequisites)
- [Quick Start](#quick-start)
- [Components](#components)
- [API Reference](#api-reference)
- [Usage Examples](#usage-examples)
- [Configuration](#configuration)
- [Development](#development)
- [Deployment](#deployment)
- [Contributing](#contributing)
- [License](#license)

---

## Overview

xxx is a distributed system that transforms arbitrary code into secure, ephemeral container executions. Built with Rust and leveraging modern cloud-native technologies, it provides:

- **Multi-language REPL execution** via containerized runtimes
- **Real-time streaming output** using Server-Sent Events (SSE)
- **Service discovery and health management** backed by etcd
- **Automatic container lifecycle management** with cleanup and resource optimization
- **Horizontal scalability** through microservice architecture
- **Production-ready observability** with structured logging and tracing

---

## Architecture

### High-Level Design

```
┌─────────────────────────────────────────────────────────────┐
│                         Clients                             │
│  ┌──────────┐                           ┌──────────┐        │
│  │   CLI    │                           │  Web UI  │        │
│  └────┬─────┘                           └────┬─────┘        │
└───────┼──────────────────────────────────────┼──────────────┘
        │                                      │
        └──────────────┬───────────────────────┘
                       │ HTTP/SSE
        ┌──────────────▼────────────────────────────────┐
        │           API Gateway Layer                   │
        │  ┌─────────────────┐   ┌──────────────────┐  │
        │  │   repl-api      │   │  container-api   │  │
        │  │   :3002         │   │  :3001          │  │
        │  └────────┬────────┘   └────────┬─────────┘  │
        └───────────┼─────────────────────┼────────────┘
                    │                     │
        ┌───────────▼─────────────────────▼────────────┐
        │         Service Registry                     │
        │  ┌──────────────────┐   ┌────────────────┐  │
        │  │ service-registry │◄──┤     etcd       │  │
        │  │      :3003       │   │  :2379/:2380   │  │
        │  └──────────────────┘   └────────────────┘  │
        └──────────────────────────────────────────────┘
                    │
        ┌───────────▼──────────────────────────────────┐
        │        Container Runtime Layer               │
        │  ┌──────────────────┐   ┌────────────────┐  │
        │  │ Fedora CoreOS    │◄──┤ Local Registry │  │
        │  │ Podman :8085     │   │    :5001       │  │
        │  └──────────────────┘   └────────────────┘  │
        └──────────────────────────────────────────────┘
```

**See [docs/architecture.md](docs/architecture.md) for detailed architecture diagrams and flow documentation.**

---

## Features

### Core Capabilities

- **Multi-Language Support**: Execute code in Python, Node.js, Ruby, Go, Rust, and more
- **Streaming Execution**: Real-time output streaming via Server-Sent Events (SSE)
- **Service Discovery**: Automatic service registration and discovery using etcd
- **Container Orchestration**: Podman-based container lifecycle management
- **Health Management**: TTL-based lease keepalives and automatic service expiration
- **Resource Optimization**: Automatic container cleanup and resource limits
- **Local Registry**: Fast image pulls through integrated Docker registry

### Security & Isolation

- **Ephemeral Containers**: Each execution runs in a fresh, isolated environment
- **Privileged Separation**: CoreOS runs in a privileged container, API services don't
- **Resource Limits**: CPU and memory constraints per service
- **Network Isolation**: Dedicated Docker network for service communication

---

## Prerequisites

### Required

- **Docker** or **Podman** with Compose support
- **Operating System**: Linux, macOS, or Windows with WSL2

### Optional (for development)

- **Rust** 1.90.0+ with Cargo (for building from source)
- **Bun** (for Web UI development)

---

## Quick Start

### 1. Clone the Repository

```bash
git clone https://github.com/geoffsee/xxx.git
cd xxx
```

### 2. Start the Platform

```bash
./scripts/run.sh
```

This will:
- Build all service containers
- Start etcd, service registry, and API services
- Initialize Fedora CoreOS with Podman
- Configure the local Docker registry

### 3. Execute Your First REPL Command

#### Using the CLI

```bash
cargo run -p cli -- repl execute \
  --language python \
  --api-url http://localhost:3002 \
  --code "for i in range(10): print(f'Hello from Python! Line {i}')"
```

#### With Streaming Output

```bash
cargo run -p cli -- repl execute \
  --language python \
  --api-url http://localhost:3002 \
  --streaming \
  --code "import time; [print(f'Tick {i}') or time.sleep(0.5) for i in range(5)]"
```

---

## Components

### Service Architecture

| Service | Port | Purpose |
|---------|------|---------|
| **repl-api** | 3002 | Translates language + code into container execution requests |
| **container-api** | 3001 | Manages container lifecycle via Podman API |
| **service-registry** | 3003 | Service discovery and health management backed by etcd |
| **coreos** | 8085 | Fedora CoreOS container exposing Podman HTTP API |
| **coreos-etcd** | 2379/2380 | Distributed key-value store for service registry |
| **registry** | 5001 | Local Docker registry for fast image pulls |

### Component Details

#### **repl-api**
- Converts user code into containerized execution requests
- Discovers `container-api` instances dynamically
- Supports both streaming (SSE) and buffered execution modes
- Handles language-to-image mapping

#### **container-api**
- Interacts with Podman's HTTP API for container operations
- Manages image pulls, container creation, execution, and cleanup
- Streams real-time stdout/stderr output
- Discovers CoreOS Podman endpoint via service registry

#### **service-registry**
- etcd-backed service discovery
- TTL-based lease management (5s keepalive interval)
- Auto-registration using `register_service!` macro
- RESTful API for registration, discovery, and health checks

#### **CLI**
- Command-line interface for REPL execution
- Supports streaming and non-streaming modes
- Built with Clap and Tokio

#### **Web UI**
- Browser-based REPL playground
- Built with Bun and modern web technologies
- Real-time execution feedback

---

## API Reference

### repl-api

#### `POST /api/repl/execute`
Execute code and return buffered output.

**Request:**
```json
{
  "language": "python",
  "code": "print('Hello, World!')"
}
```

**Response:**
```json
{
  "result": "Hello, World!\n",
  "success": true
}
```

#### `POST /api/repl/execute/stream`
Execute code with real-time streaming output (SSE).

**Request:** Same as above

**Response:** Server-Sent Events stream
```
data: Hello, World!

event: done
data: Container execution completed
```

#### `GET /api/repl/languages`
List supported languages.

**Response:**
```json
{
  "languages": ["python", "node", "ruby", "go", "rust"]
}
```

### container-api

#### `POST /api/containers/create`
Create and run a container.

**Request:**
```json
{
  "image": "python:3.11-slim",
  "command": ["python", "-c", "print('Hello')"]
}
```

**Response:**
```json
{
  "id": "container-uuid",
  "message": "Container executed successfully",
  "output": "Hello\n"
}
```

#### `POST /api/containers/create/stream`
Create and run a container with streaming output (SSE).

#### `GET /api/containers/list`
List running containers.

#### `DELETE /api/containers/{id}`
Remove a specific container.

### service-registry

#### `POST /api/registry/register`
Register a service instance.

#### `POST /api/registry/keepalive`
Refresh a service lease.

#### `POST /api/registry/deregister`
Deregister a service instance.

#### `GET /api/registry/services`
List all registered services.

#### `GET /api/registry/services/{name}`
Get instances of a specific service.

---

## Usage Examples

### Python Execution

```bash
cargo run -p cli -- repl execute \
  --language python \
  --api-url http://localhost:3002 \
  --code "
import math
print(f'Pi: {math.pi}')
print(f'Square root of 2: {math.sqrt(2)}')
"
```

### Node.js with Streaming

```bash
cargo run -p cli -- repl execute \
  --language node \
  --api-url http://localhost:3002 \
  --streaming \
  --code "
const data = [1, 2, 3, 4, 5];
data.forEach((n, i) => {
  setTimeout(() => console.log(\`Item \${i}: \${n * 2}\`), i * 500);
});
"
```

### List Available Languages

```bash
curl http://localhost:3002/api/repl/languages
```

---

## Configuration

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `SERVICE_REGISTRY_URL` | `http://service-registry:3003` | Service registry endpoint |
| `ETCD_ENDPOINTS` | `coreos-etcd:2379` | Comma-separated etcd endpoints |
| `COREOS_URL` | `http://coreos:8085` | Podman HTTP API endpoint |
| `CONTAINERS_API_URL` | - | Fallback container-api URL (bypasses discovery) |

### Resource Limits (docker compose)

Each service has defined CPU and memory limits:

```yaml
deploy:
  resources:
    limits:
      cpus: "0.5"
      memory: 1g
    reservations:
      cpus: "0.25"
      memory: 512m
```

**See [compose.yml](compose.yml) for full configuration.**

---

## Development

### Build from Source

```bash
# Build all services
cargo build --release

# Build specific service
cargo build -p repl-api --release

# Build CLI
cargo build -p cli --release
```

### Run Tests

```bash
cargo test --workspace
```

### Build Docker Images

```bash
# Build all images
docker compose build

# Build specific service
docker compose build repl-api
```

### Development Workflow

1. Make code changes in `crates/`
2. Rebuild the specific crate: `cargo build -p <crate-name>`
3. Rebuild Docker image: `docker compose build <service-name>`
4. Restart service: `docker compose up -d <service-name>`

---

## Deployment

### Production Deployment

The platform is designed for cloud deployment with support for:

- **Kubernetes**: Use Helm charts (see `crates/xxx-cloud/`)
- **Cloud Providers**: GCP, AWS, Azure support via CDKTF
- **Container Registries**: GitHub Container Registry (ghcr.io)

### Pre-built Images

Production images are available at:

```
ghcr.io/geoffsee/container-api:stable
ghcr.io/geoffsee/repl-api:stable
ghcr.io/geoffsee/service-registry:stable
ghcr.io/geoffsee/fedora-coreos:stable
ghcr.io/geoffsee/etcd:stable
ghcr.io/geoffsee/registry:stable
```

### Health Checks

All services expose health endpoints and support graceful shutdown.

```bash
# Check service registry health
curl http://localhost:3003/health

# Check etcd health
docker compose exec coreos-etcd etcdctl endpoint health
```

---

## Contributing

We welcome contributions to xxx! Please follow these guidelines:

1. **Fork** the repository
2. **Create** a feature branch: `git checkout -b feature/amazing-feature`
3. **Commit** your changes with descriptive messages
4. **Test** your changes: `cargo test --workspace`
5. **Push** to the branch: `git push origin feature/amazing-feature`
6. **Open** a Pull Request

### Code Standards

- Follow Rust best practices and idioms
- Run `cargo fmt` before committing
- Run `cargo clippy` and address warnings
- Add tests for new functionality
- Update documentation as needed

---

## License

Copyright (c) 2025 GSIO Limited. All rights reserved.

---

## Support & Contact
For issues, questions, or feature requests, please open an issue on GitHub.