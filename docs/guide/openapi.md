# OpenAPI & Swagger UI

noti-server ships with auto-generated [OpenAPI 3.1](https://spec.openapis.org/oas/v3.1.0) documentation and an interactive [Swagger UI](https://swagger.io/tools/swagger-ui/) explorer. Every API endpoint, request body, and response schema is derived directly from the Rust source code — the specification is always in sync with the running server.

## Quick Start

```bash
# Start the server
noti-server

# Open Swagger UI in your browser
open http://localhost:3000/swagger-ui/

# Fetch the raw OpenAPI JSON spec
curl http://localhost:3000/api-docs/openapi.json | jq .
```

## Endpoints

| Path | Description |
|:-----|:------------|
| `/swagger-ui/` | Interactive Swagger UI explorer |
| `/api-docs/openapi.json` | Machine-readable OpenAPI 3.1 specification (JSON) |

Both endpoints are **version-independent** — they are not prefixed with `/api/v1/` and remain stable across API versions.

::: tip
These endpoints bypass [authentication](/guide/authentication) and [rate limiting](/guide/rate-limiting) middleware, so they are always accessible.
:::

## Swagger UI

The Swagger UI provides a browser-based interface for exploring and testing the API:

- **Try it out** — execute real API requests against the running server
- **Schema inspection** — view request/response body schemas with field descriptions
- **Tag-based grouping** — endpoints are organized by functional area
- **Parameter documentation** — required vs. optional parameters, types, and examples

### API Tags

Endpoints in the OpenAPI spec are grouped into the following tags:

| Tag | Description | Example Endpoints |
|:----|:------------|:------------------|
| **Health** | Health check endpoints | `GET /health` |
| **Meta** | API version discovery and metadata | `GET /api/versions` |
| **Notifications** | Synchronous notification sending | `POST /api/v1/send`, `POST /api/v1/send/batch` |
| **Async Queue** | Asynchronous queue-based notification processing | `POST /api/v1/send/async`, `GET /api/v1/queue/stats` |
| **Status** | Delivery status tracking | `GET /api/v1/status/{notification_id}` |
| **Templates** | Message template management (CRUD) | `POST /api/v1/templates`, `GET /api/v1/templates/{name}` |
| **Providers** | Notification provider information | `GET /api/v1/providers`, `GET /api/v1/providers/{name}` |
| **Monitoring** | Metrics and monitoring | `GET /api/v1/metrics` |

## OpenAPI Specification

The JSON specification includes:

- **API info** — title, version, license (MIT), and contact link
- **Paths** — all 23 API operations with HTTP methods, parameters, request bodies, and response schemas

- **Components/Schemas** — 40+ reusable type definitions (request/response bodies, enums, nested objects)
- **Tags** — logical grouping of endpoints by functional area

### Fetching the Spec

```bash
# Full spec
curl -s http://localhost:3000/api-docs/openapi.json | jq .

# Just the paths
curl -s http://localhost:3000/api-docs/openapi.json | jq '.paths | keys'

# Just the schemas
curl -s http://localhost:3000/api-docs/openapi.json | jq '.components.schemas | keys'
```

### Example: Inspect a Specific Schema

```bash
# View the SendRequest schema
curl -s http://localhost:3000/api-docs/openapi.json \
  | jq '.components.schemas.SendRequest'
```

```json
{
  "type": "object",
  "required": ["provider", "text"],
  "properties": {
    "provider": { "type": "string", "description": "Provider name (e.g. \"slack\", \"email\", \"webhook\")." },
    "text": { "type": "string", "description": "Message body text." },
    "title": { "type": "string", "nullable": true, "description": "Optional message title/subject." },
    "format": { "type": "string", "nullable": true, "description": "Message format: \"text\", \"markdown\", or \"html\"." },
    "priority": { "type": "string", "nullable": true, "description": "Priority: \"low\", \"normal\", \"high\", \"urgent\"." },
    "config": { "type": "object", "additionalProperties": { "type": "string" } },
    "extra": { "type": "object", "additionalProperties": {} },
    "retry": { "$ref": "#/components/schemas/RetryConfig", "nullable": true }
  }
}
```

## Client SDK Generation

The OpenAPI spec can be used to auto-generate client libraries in any language:

### Using openapi-generator

```bash
# Generate a Python client
npx @openapitools/openapi-generator-cli generate \
  -i http://localhost:3000/api-docs/openapi.json \
  -g python \
  -o ./noti-python-client

# Generate a TypeScript client
npx @openapitools/openapi-generator-cli generate \
  -i http://localhost:3000/api-docs/openapi.json \
  -g typescript-fetch \
  -o ./noti-ts-client

# Generate a Go client
npx @openapitools/openapi-generator-cli generate \
  -i http://localhost:3000/api-docs/openapi.json \
  -g go \
  -o ./noti-go-client
```

### Saving the Spec for CI/CD

```bash
# Save spec to file for version control or CI pipeline
curl -s http://localhost:3000/api-docs/openapi.json > openapi.json

# Validate the spec with a linter
npx @stoplight/spectral-cli lint openapi.json
```

## Schema Components Reference

The OpenAPI spec defines these component schemas:

### Error & Common

| Schema | Description |
|:-------|:------------|
| `ApiError` | Standardized error response (`error`, `message`, optional `code`) |
| `RetryConfig` | Retry policy configuration (`max_retries`, `delay_ms`, `backoff_multiplier`, `max_delay_ms`) |

### Health & Meta

| Schema | Description |
|:-------|:------------|
| `HealthResponse` | Service health with dependency status |
| `DependencyHealth` | Per-dependency health (queue, providers) |
| `ComponentHealth` | Individual component status (`up`/`down` + detail) |
| `ApiVersion` | Version metadata (version, status, deprecated flag) |
| `ApiVersionsResponse` | List of available API versions |

### Notifications

| Schema | Description |
|:-------|:------------|
| `SendRequest` | Single notification request body |
| `SendApiResponse` | Send result with notification ID |
| `BatchSendRequest` | Multi-target batch send request |
| `BatchTarget` | Individual target in a batch (provider + config) |
| `BatchSendApiResponse` | Batch send results |
| `TargetApiResult` | Per-target result in a batch |

### Async Queue

| Schema | Description |
|:-------|:------------|
| `AsyncSendRequest` | Async enqueue request (includes `delay_seconds`, `scheduled_at`, `callback_url`) |
| `EnqueueResponse` | Enqueue confirmation with task ID |
| `BatchAsyncRequest` | Batch async enqueue request |
| `BatchEnqueueResponse` | Batch enqueue results |
| `BatchEnqueueItemResult` | Per-item enqueue result |
| `TaskInfo` | Task status and metadata |
| `StatsResponse` | Queue statistics |
| `CancelResponse` | Task cancellation result |
| `PurgeResponse` | Purge operation result |

### Status & Tracking

| Schema | Description |
|:-------|:------------|
| `StatusResponse` | Delivery records for a notification |
| `AllStatusesResponse` | Summary of all tracked statuses |
| `DeliveryRecord` | Full delivery tracking record with event history |
| `DeliveryStatus` | Enum: `pending`, `sending`, `delivered`, `failed`, `cancelled`, `read` |
| `StatusEvent` | Timestamped status transition event |
| `StatusSummary` | Aggregate counts across all statuses |
| `PurgeStatusResponse` | Status purge result (`purged`, `message`) |


### Templates

| Schema | Description |
|:-------|:------------|
| `CreateTemplateRequest` | Template creation body |
| `UpdateTemplateRequest` | Template update body |
| `TemplateResponse` | Template details (name, body, variables, defaults) |
| `TemplateListResponse` | List of template names |
| `DeleteTemplateResponse` | Deletion confirmation |
| `RenderTemplateRequest` | Variables for template rendering |
| `RenderedTemplateResponse` | Rendered text and title |

### Providers

| Schema | Description |
|:-------|:------------|
| `ProviderListResponse` | List of all providers with summary info |
| `ProviderSummary` | Provider name, scheme, description, attachment support |
| `ProviderInfo` | Detailed provider info with parameter definitions |
| `ParamInfo` | Parameter name, description, required flag, example |

### Monitoring

| Schema | Description |
|:-------|:------------|
| `MetricsResponse` | Server metrics (queue stats, provider stats, uptime, version) |
| `ProviderMetrics` | Provider-related metrics (total registered, attachment support count) |

## How It Works

noti-server uses [utoipa](https://crates.io/crates/utoipa) to derive the OpenAPI specification at compile time from Rust type annotations:

1. **Handler annotations** — Each API handler uses `#[utoipa::path(...)]` to declare its HTTP method, path, tag, request body, and response types
2. **Schema derives** — Request/response structs use `#[derive(ToSchema)]` to generate JSON Schema definitions
3. **Conditional compilation** — Core types in `noti-core` and `noti-queue` use `#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]` so the OpenAPI dependency is only included when the `openapi` feature is enabled
4. **Central aggregation** — The `ApiDoc` struct in `openapi.rs` aggregates all paths and schemas into a single OpenAPI document
5. **Swagger UI serving** — `utoipa-swagger-ui` serves the interactive UI at `/swagger-ui/` and the raw spec at `/api-docs/openapi.json`

This means the documentation is **always accurate** — if a field is added to a struct, it automatically appears in the spec. No manual synchronization required.

## Docker & Production

The OpenAPI endpoints are available in all deployment modes:

```bash
# Docker
docker run -p 3000:3000 ghcr.io/loonghao/noti-server:latest
# Access: http://localhost:3000/swagger-ui/

# Docker Compose
docker compose up -d
# Access: http://localhost:3000/swagger-ui/
```

::: warning
In production, consider whether the Swagger UI should be publicly accessible. While the spec itself is read-only and safe, exposing the "Try it out" feature to untrusted users could allow them to send test notifications. Use [authentication](/guide/authentication) and network-level controls as needed.
:::

## AI Agent Tips

- **Always use `GET /api-docs/openapi.json`** to discover the current API contract — it's the canonical source of truth
- **Parse the spec programmatically** to build request payloads matching the exact schema
- **Check `required` fields** in component schemas before constructing requests
- **Use tag filtering** to focus on specific API areas (e.g., only `Async Queue` endpoints)
