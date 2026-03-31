use std::collections::HashMap;

use axum::Json;
use axum::extract::State;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
use validator::Validate;

use noti_core::{DeliveryStatus, ProviderConfig, RetryPolicy, SendResponse};

use crate::handlers::common::{self, RetryConfig};
use crate::handlers::error::ApiError;
use crate::middleware::validated_json::ValidatedJson;
use crate::state::AppState;

/// Request body for sending a single notification.
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct SendRequest {
    /// Provider name (e.g. "slack", "email", "webhook").
    #[validate(length(min = 1, message = "provider must not be empty"))]
    pub provider: String,
    /// Provider-specific configuration values.
    #[serde(default)]
    pub config: HashMap<String, String>,
    /// Message body text.
    #[validate(length(min = 1, message = "text must not be empty"))]
    pub text: String,
    /// Optional message title/subject.
    pub title: Option<String>,
    /// Message format: "text", "markdown", or "html".
    #[serde(default)]
    pub format: Option<String>,
    /// Priority: "low", "normal", "high", "urgent".
    pub priority: Option<String>,
    /// Extra provider-specific parameters.
    #[serde(default)]
    pub extra: HashMap<String, serde_json::Value>,
    /// Retry policy configuration.
    pub retry: Option<RetryConfig>,
}

/// Request body for batch sending.
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct BatchSendRequest {
    /// List of targets to send to.
    #[validate(length(min = 1, message = "targets must not be empty"))]
    pub targets: Vec<BatchTarget>,
    /// Shared message body text.
    #[validate(length(min = 1, message = "text must not be empty"))]
    pub text: String,
    /// Optional message title.
    pub title: Option<String>,
    /// Message format.
    #[serde(default)]
    pub format: Option<String>,
    /// Priority.
    pub priority: Option<String>,
    /// Extra parameters.
    #[serde(default)]
    pub extra: HashMap<String, serde_json::Value>,
    /// Retry policy.
    pub retry: Option<RetryConfig>,
    /// Sending mode: "parallel" (default) or "failover".
    #[serde(default = "default_mode")]
    pub mode: String,
}

fn default_mode() -> String {
    "parallel".to_string()
}

/// A single target within a batch send request.
#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct BatchTarget {
    /// Provider name.
    pub provider: String,
    /// Provider-specific configuration.
    #[serde(default)]
    pub config: HashMap<String, String>,
}

/// API response for a send operation.
#[derive(Debug, Serialize, ToSchema)]
pub struct SendApiResponse {
    /// Unique notification ID for tracking.
    pub notification_id: String,
    /// Whether the send was successful.
    pub success: bool,
    /// Provider name.
    pub provider: String,
    /// Result message.
    pub message: String,
    /// HTTP status code from the provider (if applicable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_code: Option<u16>,
}

/// API response for a batch send operation.
#[derive(Debug, Serialize, ToSchema)]
pub struct BatchSendApiResponse {
    /// Unique notification ID for tracking.
    pub notification_id: String,
    /// Sending mode used.
    pub mode: String,
    /// Per-target results.
    pub results: Vec<TargetApiResult>,
    /// Number of successes.
    pub success_count: usize,
    /// Number of failures.
    pub failure_count: usize,
    /// Total duration in milliseconds.
    pub total_duration_ms: u64,
}

/// Per-target result within a batch response.
#[derive(Debug, Serialize, ToSchema)]
pub struct TargetApiResult {
    pub provider: String,
    pub success: bool,
    pub message: String,
    pub attempts: u32,
}

/// Send a single notification synchronously.
#[utoipa::path(
    post,
    path = "/api/v1/send",
    tag = "Notifications",
    request_body = SendRequest,
    responses(
        (status = 200, description = "Notification sent", body = SendApiResponse),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 404, description = "Provider not found", body = ApiError),
    )
)]
pub async fn send_notification(
    State(state): State<AppState>,
    ValidatedJson(req): ValidatedJson<SendRequest>,
) -> Result<Json<SendApiResponse>, ApiError> {
    let provider = state
        .registry
        .get_by_name(&req.provider)
        .ok_or_else(|| ApiError::not_found(format!("provider '{}' not found", req.provider)))?;

    let config = ProviderConfig {
        values: req.config,
    };

    if let Err(e) = provider.validate_config(&config) {
        return Err(ApiError::bad_request(e.to_string()));
    }

    let msg = common::build_message(
        &req.text,
        req.title.as_deref(),
        req.format.as_deref(),
        req.priority.as_deref(),
        &req.extra,
    );

    let notification_id = Uuid::new_v4().to_string();
    let provider_name = provider.name().to_string();

    // Track the delivery
    state
        .status_tracker
        .track(&notification_id, &provider_name)
        .await;
    state
        .status_tracker
        .update_status(
            &notification_id,
            &provider_name,
            DeliveryStatus::Sending,
            None,
        )
        .await;

    let policy = common::build_retry_policy(req.retry.as_ref(), RetryPolicy::none());

    let result: Result<SendResponse, _> = if policy.max_retries == 0 {
        provider.send(&msg, &config).await
    } else {
        let outcome =
            noti_core::send_with_retry(provider.as_ref(), &msg, &config, &policy).await;
        outcome.result
    };

    match result {
        Ok(resp) => {
            state
                .status_tracker
                .update_status(
                    &notification_id,
                    &provider_name,
                    DeliveryStatus::Delivered,
                    Some(resp.message.clone()),
                )
                .await;

            Ok(Json(SendApiResponse {
                notification_id,
                success: resp.success,
                provider: resp.provider,
                message: resp.message,
                status_code: resp.status_code,
            }))
        }
        Err(e) => {
            state
                .status_tracker
                .update_status(
                    &notification_id,
                    &provider_name,
                    DeliveryStatus::Failed,
                    Some(e.to_string()),
                )
                .await;

            Ok(Json(SendApiResponse {
                notification_id,
                success: false,
                provider: provider_name,
                message: e.to_string(),
                status_code: None,
            }))
        }
    }
}

/// Send to multiple providers in parallel or failover mode.
#[utoipa::path(
    post,
    path = "/api/v1/send/batch",
    tag = "Notifications",
    request_body = BatchSendRequest,
    responses(
        (status = 200, description = "Batch send completed", body = BatchSendApiResponse),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 404, description = "Provider not found", body = ApiError),
    )
)]
pub async fn send_batch(
    State(state): State<AppState>,
    ValidatedJson(req): ValidatedJson<BatchSendRequest>,
) -> Result<Json<BatchSendApiResponse>, ApiError> {
    // Validate all providers exist
    let mut providers = Vec::new();
    let mut configs = Vec::new();

    for target in &req.targets {
        let provider = state
            .registry
            .get_by_name(&target.provider)
            .ok_or_else(|| {
                ApiError::not_found(format!("provider '{}' not found", target.provider))
            })?;
        providers.push(provider.clone());
        configs.push(ProviderConfig {
            values: target.config.clone(),
        });
    }

    let msg = common::build_message(
        &req.text,
        req.title.as_deref(),
        req.format.as_deref(),
        req.priority.as_deref(),
        &req.extra,
    );

    let notification_id = Uuid::new_v4().to_string();
    let policy = common::build_retry_policy(req.retry.as_ref(), RetryPolicy::none());

    // Track all targets
    for p in &providers {
        state
            .status_tracker
            .track(&notification_id, p.name())
            .await;
    }

    // Build send targets
    let send_targets: Vec<noti_core::SendTarget<'_>> = providers
        .iter()
        .zip(configs.iter())
        .map(|(p, c)| noti_core::SendTarget::new(p.as_ref(), c))
        .collect();

    let batch_result = if req.mode == "failover" {
        noti_core::send_failover(&send_targets, &msg, &policy).await
    } else {
        noti_core::send_batch(&send_targets, &msg, &policy).await
    };

    // Map results and update statuses
    let mut api_results = Vec::new();
    for target_result in &batch_result.results {
        let success = target_result.is_success();
        let (message, status) = if success {
            let msg = target_result
                .outcome
                .result
                .as_ref()
                .map(|r| r.message.clone())
                .unwrap_or_default();
            (msg, DeliveryStatus::Delivered)
        } else {
            let msg = match &target_result.outcome.result {
                Ok(r) => r.message.clone(),
                Err(e) => e.to_string(),
            };
            (msg, DeliveryStatus::Failed)
        };

        state
            .status_tracker
            .update_status(
                &notification_id,
                &target_result.provider_name,
                status,
                Some(message.clone()),
            )
            .await;

        api_results.push(TargetApiResult {
            provider: target_result.provider_name.clone(),
            success,
            message,
            attempts: target_result.outcome.attempts,
        });
    }

    Ok(Json(BatchSendApiResponse {
        notification_id,
        mode: req.mode,
        results: api_results,
        success_count: batch_result.success_count(),
        failure_count: batch_result.failure_count(),
        total_duration_ms: batch_result.total_duration.as_millis() as u64,
    }))
}
