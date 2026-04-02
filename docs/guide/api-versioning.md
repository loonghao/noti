# API Versioning

noti-server uses **URL path versioning** to ensure backward compatibility as the API evolves. All versioned endpoints live under `/api/v{N}/`, allowing multiple API versions to coexist.

## Version Discovery

Query the version-independent endpoint to discover available API versions:

```bash
curl http://localhost:3000/api/versions
```

```json
{
  "versions": [
    {
      "version": "v1",
      "status": "stable",
      "deprecated": false
    }
  ],
  "latest": "v1"
}
```

### Version Lifecycle

Each version has a lifecycle status:

| Status | Meaning |
|--------|---------|
| `stable` | Production-ready, fully supported |
| `beta` | Feature-complete but may change |
| `deprecated` | Still functional, scheduled for removal |

## Current Versions

### v1 (Stable)

All v1 endpoints are prefixed with `/api/v1/`:

| Category | Endpoints |
|----------|-----------|
| **Notifications** | `POST /api/v1/send`, `POST /api/v1/send/batch` |
| **Async Queue** | `POST /api/v1/send/async`, `POST /api/v1/send/async/batch` |
| **Status** | `GET /api/v1/status`, `GET /api/v1/status/{id}` |
| **Templates** | `GET/POST /api/v1/templates`, `GET/PUT/DELETE /api/v1/templates/{name}` |
| **Providers** | `GET /api/v1/providers`, `GET /api/v1/providers/{name}` |
| **Queue** | `GET /api/v1/queue/stats`, `GET /api/v1/queue/tasks`, etc. |
| **Metrics** | `GET /api/v1/metrics` |

### Version-Independent Endpoints

These endpoints are not versioned and remain stable across all API versions:

| Endpoint | Description |
|----------|-------------|
| `GET /health` | Health check with dependency status |
| `GET /api/versions` | API version discovery |
| `GET /swagger-ui/` | Interactive API documentation |
| `GET /api-docs/openapi.json` | OpenAPI 3.1 specification |

## Architecture

The router uses [`Router::nest`](https://docs.rs/axum/latest/axum/struct.Router.html#method.nest) to mount each API version under its prefix:

```rust
Router::new()
    .route("/health", get(health_check))
    .route("/api/versions", get(list_api_versions))
    .nest("/api/v1", build_v1_routes())
    // Future: .nest("/api/v2", build_v2_routes())
    .with_state(state)
```

This design means:
- **v1 handlers are isolated** — they only define paths relative to their version root (e.g. `/send` instead of `/api/v1/send`)
- **Adding v2 is a single line** — mount a new set of routes under `/api/v2`
- **No breaking changes** — existing v1 clients continue working unchanged

## Best Practices for API Consumers

1. **Always use the full versioned path** (e.g. `/api/v1/send`), never the bare path
2. **Check `/api/versions`** at startup to verify the server supports your expected version
3. **Monitor the `deprecated` flag** — when a version is deprecated, migrate before it's removed
4. **Use the OpenAPI spec** at `/api-docs/openapi.json` for auto-generated client SDKs

## Deprecation Policy

When a new API version is introduced:

1. The previous version is marked as `deprecated: true` in `/api/versions`
2. Deprecated versions remain functional for **at least 6 months**
3. A `Sunset` HTTP header is added to deprecated version responses with the removal date
4. After the sunset date, the deprecated version returns `410 Gone`
