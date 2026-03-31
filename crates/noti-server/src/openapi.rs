//! OpenAPI specification generation for the noti-server API.
//!
//! Uses [utoipa](https://crates.io/crates/utoipa) to auto-generate an OpenAPI 3.1
//! specification from handler annotations and schema derives.

use utoipa::OpenApi;

use crate::handlers::{
    common::RetryConfig,
    error::ApiError,
    health::{self, ComponentHealth, DependencyHealth, HealthResponse},
    metrics::{self, MetricsResponse, ProviderMetrics},
    providers::{self, ParamInfo, ProviderInfo, ProviderListResponse, ProviderSummary},
    queue::{
        self, AsyncSendRequest, BatchAsyncItem, BatchAsyncRequest, BatchEnqueueItemResult,
        BatchEnqueueResponse, CancelResponse, EnqueueResponse, PurgeResponse, StatsResponse,
        TaskInfo,
    },
    send::{
        self, BatchSendApiResponse, BatchSendRequest, BatchTarget, SendApiResponse, SendRequest,
        TargetApiResult,
    },
    status::{self, AllStatusesResponse, StatusResponse},
    templates::{
        self, CreateTemplateRequest, DeleteTemplateResponse, RenderTemplateRequest,
        RenderedTemplateResponse, TemplateListResponse, TemplateResponse, UpdateTemplateRequest,
    },
};

/// Auto-generated OpenAPI documentation for the noti notification service.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "noti — Notification Service API",
        version = env!("CARGO_PKG_VERSION"),
        description = "A unified notification service supporting 130+ providers with sync/async delivery, \
            message templates, priority queuing, and delivery status tracking.",
        license(name = "MIT", url = "https://github.com/loonghao/noti/blob/main/LICENSE"),
        contact(name = "noti", url = "https://github.com/loonghao/noti"),
    ),
    tags(
        (name = "Health", description = "Health check endpoints"),
        (name = "Notifications", description = "Synchronous notification sending"),
        (name = "Async Queue", description = "Asynchronous queue-based notification processing"),
        (name = "Status", description = "Delivery status tracking"),
        (name = "Templates", description = "Message template management"),
        (name = "Providers", description = "Notification provider information"),
        (name = "Monitoring", description = "Metrics and monitoring"),
    ),
    paths(
        // Health
        health::health_check,
        // Notifications
        send::send_notification,
        send::send_batch,
        // Async Queue
        queue::send_async,
        queue::send_async_batch,
        queue::get_task,
        queue::list_tasks,
        queue::get_stats,
        queue::cancel_task,
        queue::purge_tasks,
        // Status
        status::get_status,
        status::get_all_statuses,
        // Templates
        templates::create_template,
        templates::list_templates,
        templates::get_template,
        templates::update_template,
        templates::delete_template,
        templates::render_template,
        // Providers
        providers::list_providers,
        providers::get_provider,
        // Monitoring
        metrics::get_metrics,
    ),
    components(schemas(
        // Error
        ApiError,
        // Health
        HealthResponse, DependencyHealth, ComponentHealth,
        // Common
        RetryConfig,
        // Send
        SendRequest, SendApiResponse,
        BatchSendRequest, BatchTarget, BatchSendApiResponse, TargetApiResult,
        // Queue
        AsyncSendRequest, EnqueueResponse,
        BatchAsyncRequest, BatchAsyncItem, BatchEnqueueResponse, BatchEnqueueItemResult,
        TaskInfo, StatsResponse, CancelResponse, PurgeResponse,
        // Status
        StatusResponse, AllStatusesResponse,
        // Templates
        CreateTemplateRequest, TemplateResponse, TemplateListResponse,
        UpdateTemplateRequest, DeleteTemplateResponse,
        RenderTemplateRequest, RenderedTemplateResponse,
        // Providers
        ProviderListResponse, ProviderSummary, ProviderInfo, ParamInfo,
        // Metrics
        MetricsResponse, ProviderMetrics,
        // Core types (from noti-core with openapi feature)
        noti_core::DeliveryRecord, noti_core::DeliveryStatus,
        noti_core::StatusEvent, noti_core::StatusSummary,
    ))
)]
pub struct ApiDoc;
