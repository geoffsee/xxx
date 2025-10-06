# xxx Architecture and System Diagrams

This document explains how the project’s services fit together and how requests flow through the system.

## Overview

- Purpose: Provide a simple REPL-as-a-service backed by ephemeral containers run via Podman on a Fedora CoreOS host.
- Key pieces:
  - `service-registry`: HTTP service that stores service registrations in etcd and provides discovery + lease keepalives.
  - `container-api`: HTTP service that pulls images, runs containers, captures output, and cleans up.
  - `repl-api`: HTTP service that turns user code into container runs via `container-api`.
  - `coreos`: Fedora CoreOS container exposing Podman’s remote HTTP API (socket on `:8085`).
  - `coreos-etcd`: etcd backend for `service-registry`.
  - `registry`: Local Docker registry for faster, local image pulls.
  - `cli` and `ui`: Clients that call the APIs.

## High-Level Architecture

```mermaid
flowchart LR
    subgraph Clients
        CLI[CLI] -->|HTTP| CAPI_H["container-api (host :3001)"]
        UI[UI App] -->|HTTP| RAPI_H["repl-api (host :3002)"]
    end

    subgraph coreos-net["Docker Compose Network: coreos-net"]
        SR["service-registry<br/>Axum + etcd client<br/>:3003"]
        ETCD["etcd<br/>:2379/:2380"]
        CAPI["container-api<br/>Axum + Podman client<br/>:3000"]
        RAPI["repl-api<br/>Axum<br/>:3001"]
        COREOS["Fedora CoreOS<br/>Podman HTTP socket<br/>:8085"]
        REG["Local Registry<br/>registry:5000"]
    end

%% Host port mappings
    CAPI_H --- CAPI
    RAPI_H --- RAPI

%% Service registration/discovery
    CAPI -- register + keepalive --> SR
    RAPI -- register + keepalive --> SR
    SR -- stores leases/keys --> ETCD

%% REPL flow
    RAPI -- discover container-api --> SR
    RAPI -- call /api/containers/* --> CAPI

%% Container flow
    CAPI -- discover coreos --> SR
    CAPI -- Podman HTTP API --> COREOS
    COREOS -- image pulls --> REG

```

Notes:
- `service-registry` persists service instances in etcd under keys `/services/{name}/{id}` with a TTL lease. Clients keep leases alive via `/api/registry/keepalive`.
- `container-api` and `repl-api` auto-register on startup using the `register_service!` macro which delegates to `bootstrap_service`.
- `repl-api` discovers `container-api` dynamically; `container-api` discovers the Podman endpoint (`coreos`) dynamically. Both fall back to env vars when service discovery is unavailable.

## Service Registration Flow

```mermaid
sequenceDiagram
  participant Svc as Service (container-api/repl-api)
  participant Macro as register_service! macro
  participant Boot as bootstrap_service()
  participant SR as service-registry (HTTP :3003)
  participant ETCD as etcd

  Svc->>Macro: register_service!("name", "address", port)
  Macro->>Boot: bootstrap_service(name, address, port)
  Boot->>SR: POST /api/registry/register { ServiceInfo }
  SR->>ETCD: lease_grant(TTL) + put /services/name/id
  SR-->>Boot: { lease_id }
  Note right of Boot: Spawn background task
  loop every 5s
    Boot->>SR: POST /api/registry/keepalive { lease_id }
  end
```

- Auto-registration can also occur in `service-registry` for `coreos` when `COREOS_URL` is set (extracts host/port and registers).

## REPL Execution Flow

### Standard (Non-Streaming) Execution

```mermaid
sequenceDiagram
  participant User as User (CLI/UI)
  participant RAPI as repl-api
  participant SR as service-registry
  participant CAPI as container-api
  participant SR2 as service-registry
  participant CORE as Fedora CoreOS (Podman)
  participant REG as Local Registry

  User->>RAPI: POST /api/repl/execute {language, code}
  RAPI->>SR: GET /api/registry/services/container-api
  SR-->>RAPI: [{address, port, ...}]
  RAPI->>CAPI: POST /api/containers/create {image, command}
  CAPI->>SR2: GET /api/registry/services/coreos
  SR2-->>CAPI: [{address, port, ...}]
  CAPI->>CORE: Podman: pull(image)
  CORE->>REG: Fetch image layers
  CORE-->>CAPI: stream pull progress
  CAPI->>CORE: create/start container(command)
  CORE-->>CAPI: run complete + logs
  CAPI-->>RAPI: {id, message, output}
  RAPI-->>User: {result, success:true}
```

### Streaming Execution (SSE)

The CLI and other clients now use streaming by default for real-time output:

```mermaid
sequenceDiagram
  participant User as User (CLI/UI)
  participant RAPI as repl-api
  participant SR as service-registry
  participant CAPI as container-api
  participant SR2 as service-registry
  participant CORE as Fedora CoreOS (Podman)
  participant REG as Local Registry

  User->>RAPI: POST /api/repl/execute/stream {language, code}
  RAPI->>SR: GET /api/registry/services/container-api
  SR-->>RAPI: [{address, port, ...}]
  RAPI->>CAPI: POST /api/containers/create/stream {image, command}
  CAPI->>SR2: GET /api/registry/services/coreos
  SR2-->>CAPI: [{address, port, ...}]
  CAPI->>CORE: Podman: pull(image) + attach
  CORE->>REG: Fetch image layers
  CAPI->>CORE: start container(command)
  loop Real-time output
    CORE-->>CAPI: SSE: stdout/stderr chunks
    CAPI-->>RAPI: SSE: forward chunks
    RAPI-->>User: SSE: stream output
  end
  CAPI->>CORE: wait + cleanup
  CAPI-->>RAPI: SSE: event=done
  RAPI-->>User: SSE: event=done
```

Streaming features:
- Uses Server-Sent Events (SSE) for real-time output delivery
- Container output is streamed as it's generated (no buffering)
- Errors are prefixed with `ERROR:` in the stream
- Stream ends with `event:done` when container execution completes
- Automatic cleanup after container finishes

Error handling:
- If discovery fails, services fall back to env vars (`CONTAINERS_API_URL`, `COREOS_URL`).
- Pull or run errors are propagated back to callers with `500` and explanatory messages.
- For streaming endpoints, errors are sent as SSE events prefixed with `ERROR:`

## Container Lifecycle (container-api)

### Standard (Non-Streaming) Flow

```mermaid
flowchart TD
  A[Request: image + command] --> B[Pull image]
  B -->|stream progress| C{Pull success?}
  C -- no --> E[Return 500 + error]
  C -- yes --> F[Create container]
  F -->|if ok| G[Start container]
  F -->|if fail| E
  G --> H[Wait for exit]
  H --> I[Fetch logs stdout/stderr]
  I --> J[Remove container]
  J --> K[Return 200 + id + output]
```

### Streaming Flow (SSE)

```mermaid
flowchart TD
  A[Request: image + command] --> B[Pull image]
  B -->|monitor progress| C{Pull success?}
  C -- no --> E[SSE: ERROR event]
  C -- yes --> F[Create container]
  F -->|if fail| E
  F -->|if ok| G[Attach to container]
  G --> H[Start container]
  H --> I[Stream stdout/stderr in real-time]
  I -->|SSE: data events| J{Container running?}
  J -- yes --> I
  J -- no --> K[Wait for exit]
  K --> L[Remove container]
  L --> M[SSE: event=done]
```

## Service Registry Data Model

```mermaid
classDiagram
  class ServiceStatus {
    <<enum>>
    +Healthy
    +Unhealthy
    +Starting
    +Stopping
  }

  class ServiceInfo {
    +String name
    +String id
    +String address
    +u16 port
    +ServiceStatus status
    +HashMap~String,String~ metadata
    +String version
    +service_key() String
  }

  class ServiceRegistry {
    +new(endpoints: Vec~String~, lease_ttl: Option~i64~) Result
    +register(service: &ServiceInfo) i64
    +keep_alive(lease_id: i64) ()
    +deregister(service: &ServiceInfo) ()
    +get_service(name: &str, id: &str) ServiceInfo
    +get_services(name: &str) Vec~ServiceInfo~
    +get_all_services() Vec~ServiceInfo~
    +watch_service(name: &str) ()
  }

  ServiceInfo --> ServiceStatus
  ServiceRegistry ..> ServiceInfo : stores in etcd /services/{name}/{id}
```

## Deployment View (docker compose)

```mermaid
flowchart LR
  subgraph Host
    subgraph Compose[compose.yml]
      CAPIH[container-api\nport 3001->3000]
      RAPIH[repl-api\nport 3002->3001]
      SRH[service-registry\nport 3003]
      REGH[registry\nport 5001->5000]
      ETCDH[coreos-etcd\n2379,2380]
      CORE[coreos\nPodman :8085]
    end
  end

  CAPIH -- SERVICE_REGISTRY_URL --> SRH
  RAPIH -- SERVICE_REGISTRY_URL --> SRH
  SRH -- ETCD_ENDPOINTS --> ETCDH
  SRH -- COREOS_URL --> CORE
  CAPIH -- discovers --> CORE
  CORE -- pulls --> REGH
```

- Network: All services join `coreos-net` for intra-service communication.
- Ports: Host-to-container mappings provide local access for development/testing.

## API Endpoints

- `service-registry`:
  - `POST /api/registry/register` → `{ lease_id }`
  - `POST /api/registry/keepalive` → `200 OK`
  - `POST /api/registry/deregister` → `200 OK`
  - `GET /api/registry/services` → `ServiceInfo[]`
  - `GET /api/registry/services/{name}` → `ServiceInfo[]`
- `container-api`:
  - `GET  /api/containers/list` → `string[][]`
  - `POST /api/containers/create` → `{ id, message, output? }`
  - `POST /api/containers/create/stream` → SSE stream (real-time output)
  - `DELETE /api/containers/{id}` → `{ id, message }`
- `repl-api`:
  - `GET  /api/repl/languages` → `{ languages: string[] }`
  - `POST /api/repl/execute` → `{ result, success }`
  - `POST /api/repl/execute/stream` → SSE stream (real-time output)

## Configuration

- `SERVICE_REGISTRY_URL`: Base URL for service-registry (default `http://service-registry:3003`).
- `ETCD_ENDPOINTS`: Comma-separated etcd endpoints for service-registry.
- `COREOS_URL`: Base URL for Podman on CoreOS (default `http://coreos:8085`).
- `CONTAINERS_API_URL`: Base URL used by `repl-api` when discovery is unavailable.

## How Things Fit Together

- Services register themselves and keep leases alive, enabling lightweight discovery and health indication.
- `repl-api` converts language + code into an image + command, then delegates execution to `container-api`.
- `container-api` pulls the requested image and executes the command on CoreOS’s Podman, streams results, and performs cleanup.
- etcd + leases ensure that stale service entries expire automatically if a service dies.