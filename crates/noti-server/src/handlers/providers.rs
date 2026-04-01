use axum::Json;
use axum::extract::{Path, State};
use serde::Serialize;
use utoipa::ToSchema;

use crate::handlers::common;
use crate::handlers::error::ApiError;
use crate::state::AppState;

/// Provider summary info.
#[derive(Debug, Serialize, ToSchema)]
pub struct ProviderInfo {
    pub name: String,
    pub scheme: String,
    pub description: String,
    pub example_url: String,
    pub supports_attachments: bool,
    pub params: Vec<ParamInfo>,
}

/// Parameter definition info.
#[derive(Debug, Serialize, ToSchema)]
pub struct ParamInfo {
    pub name: String,
    pub description: String,
    pub required: bool,
    pub example: Option<String>,
}

/// List providers response.
#[derive(Debug, Serialize, ToSchema)]
pub struct ProviderListResponse {
    pub providers: Vec<ProviderSummary>,
    pub total: usize,
}

/// Provider summary for list endpoint.
#[derive(Debug, Serialize, ToSchema)]
pub struct ProviderSummary {
    pub name: String,
    pub scheme: String,
    pub description: String,
    pub supports_attachments: bool,
}

/// List all available notification providers.
#[utoipa::path(
    get,
    path = "/api/v1/providers",
    tag = "Providers",
    responses(
        (status = 200, description = "List of all providers", body = ProviderListResponse)
    )
)]
pub async fn list_providers(State(state): State<AppState>) -> Json<ProviderListResponse> {
    let mut providers: Vec<ProviderSummary> = state
        .registry
        .all_providers()
        .into_iter()
        .map(|p| ProviderSummary {
            name: p.name().to_string(),
            scheme: p.url_scheme().to_string(),
            description: p.description().to_string(),
            supports_attachments: p.supports_attachments(),
        })
        .collect();

    providers.sort_by(|a, b| a.name.cmp(&b.name));
    let total = providers.len();

    Json(ProviderListResponse { providers, total })
}

/// Get detailed info about a specific provider.
#[utoipa::path(
    get,
    path = "/api/v1/providers/{name}",
    tag = "Providers",
    params(("name" = String, Path, description = "Provider name")),
    responses(
        (status = 200, description = "Provider details", body = ProviderInfo),
        (status = 404, description = "Provider not found", body = ApiError),
    )
)]
pub async fn get_provider(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<ProviderInfo>, ApiError> {
    let provider = common::require_provider(&state.registry, &name)?;

    let params = provider
        .params()
        .into_iter()
        .map(|p| ParamInfo {
            name: p.name,
            description: p.description,
            required: p.required,
            example: p.example,
        })
        .collect();

    Ok(Json(ProviderInfo {
        name: provider.name().to_string(),
        scheme: provider.url_scheme().to_string(),
        description: provider.description().to_string(),
        example_url: provider.example_url().to_string(),
        supports_attachments: provider.supports_attachments(),
        params,
    }))
}
