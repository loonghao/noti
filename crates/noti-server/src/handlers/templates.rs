use std::collections::HashMap;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use validator::Validate;

use noti_core::MessageTemplate;

use crate::handlers::error::ApiError;
use crate::middleware::validated_json::ValidatedJson;
use crate::state::AppState;

/// Request body for creating a template.
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateTemplateRequest {
    /// Template name (unique identifier).
    #[validate(length(min = 1, message = "name must not be empty"))]
    pub name: String,
    /// Template body with {{variable}} placeholders.
    #[validate(length(min = 1, message = "body must not be empty"))]
    pub body: String,
    /// Optional title template.
    pub title: Option<String>,
    /// Default values for variables.
    #[serde(default)]
    pub defaults: HashMap<String, String>,
}

/// Request body for rendering a template.
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct RenderTemplateRequest {
    /// Variable values for rendering.
    pub variables: HashMap<String, String>,
}

/// Template info response.
#[derive(Debug, Serialize, ToSchema)]
pub struct TemplateResponse {
    pub name: String,
    pub body: String,
    pub title: Option<String>,
    pub variables: Vec<String>,
    pub defaults: HashMap<String, String>,
}

/// Rendered template response.
#[derive(Debug, Serialize, ToSchema)]
pub struct RenderedTemplateResponse {
    pub text: String,
    pub title: Option<String>,
}

/// Request body for updating a template.
#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateTemplateRequest {
    /// New template body with {{variable}} placeholders.
    pub body: Option<String>,
    /// New title template (set to null to remove).
    pub title: Option<String>,
    /// Default values to merge (existing defaults not mentioned are preserved).
    #[serde(default)]
    pub defaults: HashMap<String, String>,
}

/// Template list response.
#[derive(Debug, Serialize, ToSchema)]
pub struct TemplateListResponse {
    pub templates: Vec<String>,
    pub total: usize,
}

/// Response for deleting a template.
#[derive(Debug, Serialize, ToSchema)]
pub struct DeleteTemplateResponse {
    pub name: String,
    pub deleted: bool,
    pub message: String,
}

/// Create a new message template.
#[utoipa::path(
    post,
    path = "/api/v1/templates",
    tag = "Templates",
    request_body = CreateTemplateRequest,
    responses(
        (status = 201, description = "Template created", body = TemplateResponse),
    )
)]
pub async fn create_template(
    State(state): State<AppState>,
    ValidatedJson(req): ValidatedJson<CreateTemplateRequest>,
) -> Result<(StatusCode, Json<TemplateResponse>), ApiError> {
    let mut template = MessageTemplate::new(&req.name, &req.body);

    if let Some(title) = &req.title {
        template = template.with_title(title);
    }

    for (key, value) in &req.defaults {
        template = template.with_default(key, value);
    }

    let variables = template.variables();
    let defaults = template.defaults.clone();
    let name = template.name.clone();
    let body = template.body.clone();
    let title = template.title.clone();

    let mut registry = state.template_registry.write().await;
    registry.register(template);

    Ok((
        StatusCode::CREATED,
        Json(TemplateResponse {
            name,
            body,
            title,
            variables,
            defaults,
        }),
    ))
}

/// List all registered templates.
#[utoipa::path(
    get,
    path = "/api/v1/templates",
    tag = "Templates",
    responses(
        (status = 200, description = "Template list", body = TemplateListResponse)
    )
)]
pub async fn list_templates(State(state): State<AppState>) -> Json<TemplateListResponse> {
    let registry = state.template_registry.read().await;
    let names: Vec<String> = registry.names().into_iter().map(|s| s.to_string()).collect();
    let total = names.len();

    Json(TemplateListResponse {
        templates: names,
        total,
    })
}

/// Get a specific template by name.
#[utoipa::path(
    get,
    path = "/api/v1/templates/{name}",
    tag = "Templates",
    params(("name" = String, Path, description = "Template name")),
    responses(
        (status = 200, description = "Template found", body = TemplateResponse),
        (status = 404, description = "Template not found", body = ApiError),
    )
)]
pub async fn get_template(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<TemplateResponse>, ApiError> {
    let registry = state.template_registry.read().await;
    let template = registry
        .get(&name)
        .ok_or_else(|| ApiError::not_found(format!("template '{}' not found", name)))?;

    Ok(Json(TemplateResponse {
        name: template.name.clone(),
        body: template.body.clone(),
        title: template.title.clone(),
        variables: template.variables(),
        defaults: template.defaults.clone(),
    }))
}

/// Render a template with variable substitution.
#[utoipa::path(
    post,
    path = "/api/v1/templates/{name}/render",
    tag = "Templates",
    params(("name" = String, Path, description = "Template name")),
    request_body = RenderTemplateRequest,
    responses(
        (status = 200, description = "Template rendered", body = RenderedTemplateResponse),
        (status = 400, description = "Missing required variables", body = ApiError),
        (status = 404, description = "Template not found", body = ApiError),
    )
)]
pub async fn render_template(
    State(state): State<AppState>,
    Path(name): Path<String>,
    ValidatedJson(req): ValidatedJson<RenderTemplateRequest>,
) -> Result<Json<RenderedTemplateResponse>, ApiError> {
    let registry = state.template_registry.read().await;
    let template = registry
        .get(&name)
        .ok_or_else(|| ApiError::not_found(format!("template '{}' not found", name)))?;

    if let Err(e) = template.validate_vars(&req.variables) {
        return Err(ApiError::bad_request(e.to_string()));
    }

    let text = template.render_body(&req.variables);
    let title = template.render_title(&req.variables);

    Ok(Json(RenderedTemplateResponse { text, title }))
}

/// Update an existing template.
#[utoipa::path(
    put,
    path = "/api/v1/templates/{name}",
    tag = "Templates",
    params(("name" = String, Path, description = "Template name")),
    request_body = UpdateTemplateRequest,
    responses(
        (status = 200, description = "Template updated", body = TemplateResponse),
        (status = 404, description = "Template not found", body = ApiError),
    )
)]
pub async fn update_template(
    State(state): State<AppState>,
    Path(name): Path<String>,
    ValidatedJson(req): ValidatedJson<UpdateTemplateRequest>,
) -> Result<Json<TemplateResponse>, ApiError> {
    let mut registry = state.template_registry.write().await;

    let existing = registry
        .get(&name)
        .ok_or_else(|| ApiError::not_found(format!("template '{}' not found", name)))?;

    // Build updated template
    let new_body = req.body.as_deref().unwrap_or(&existing.body);
    let mut updated = MessageTemplate::new(&name, new_body);

    // Handle title: use new if provided, otherwise keep existing
    let new_title = req.title.as_deref().or(existing.title.as_deref());
    if let Some(t) = new_title {
        updated = updated.with_title(t);
    }

    // Merge defaults: start with existing, overwrite with new
    let mut merged_defaults = existing.defaults.clone();
    for (k, v) in &req.defaults {
        merged_defaults.insert(k.clone(), v.clone());
    }
    for (k, v) in &merged_defaults {
        updated = updated.with_default(k, v);
    }

    let response = TemplateResponse {
        name: updated.name.clone(),
        body: updated.body.clone(),
        title: updated.title.clone(),
        variables: updated.variables(),
        defaults: updated.defaults.clone(),
    };

    registry.register(updated);

    Ok(Json(response))
}

/// Delete a template by name.
#[utoipa::path(
    delete,
    path = "/api/v1/templates/{name}",
    tag = "Templates",
    params(("name" = String, Path, description = "Template name")),
    responses(
        (status = 200, description = "Template deleted", body = DeleteTemplateResponse),
        (status = 404, description = "Template not found", body = ApiError),
    )
)]
pub async fn delete_template(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<DeleteTemplateResponse>, ApiError> {
    let mut registry = state.template_registry.write().await;

    match registry.remove(&name) {
        Some(_) => Ok(Json(DeleteTemplateResponse {
            name,
            deleted: true,
            message: "Template deleted successfully".to_string(),
        })),
        None => Err(ApiError::not_found(format!(
            "template '{}' not found",
            name
        ))),
    }
}
