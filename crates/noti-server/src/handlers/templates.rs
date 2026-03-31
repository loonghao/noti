use std::collections::HashMap;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};

use noti_core::MessageTemplate;

use crate::handlers::error::ApiError;
use crate::state::AppState;

/// Request body for creating a template.
#[derive(Debug, Deserialize)]
pub struct CreateTemplateRequest {
    /// Template name (unique identifier).
    pub name: String,
    /// Template body with {{variable}} placeholders.
    pub body: String,
    /// Optional title template.
    pub title: Option<String>,
    /// Default values for variables.
    #[serde(default)]
    pub defaults: HashMap<String, String>,
}

/// Request body for rendering a template.
#[derive(Debug, Deserialize)]
pub struct RenderTemplateRequest {
    /// Variable values for rendering.
    pub variables: HashMap<String, String>,
}

/// Template info response.
#[derive(Debug, Serialize)]
pub struct TemplateResponse {
    pub name: String,
    pub body: String,
    pub title: Option<String>,
    pub variables: Vec<String>,
    pub defaults: HashMap<String, String>,
}

/// Rendered template response.
#[derive(Debug, Serialize)]
pub struct RenderedTemplateResponse {
    pub text: String,
    pub title: Option<String>,
}

/// Request body for updating a template.
#[derive(Debug, Deserialize)]
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
#[derive(Debug, Serialize)]
pub struct TemplateListResponse {
    pub templates: Vec<String>,
    pub total: usize,
}

/// Response for deleting a template.
#[derive(Debug, Serialize)]
pub struct DeleteTemplateResponse {
    pub name: String,
    pub deleted: bool,
    pub message: String,
}

/// POST /api/v1/templates — Create a new message template.
pub async fn create_template(
    State(state): State<AppState>,
    Json(req): Json<CreateTemplateRequest>,
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

/// GET /api/v1/templates — List all registered templates.
pub async fn list_templates(State(state): State<AppState>) -> Json<TemplateListResponse> {
    let registry = state.template_registry.read().await;
    let names: Vec<String> = registry.names().into_iter().map(|s| s.to_string()).collect();
    let total = names.len();

    Json(TemplateListResponse {
        templates: names,
        total,
    })
}

/// GET /api/v1/templates/:name — Get a specific template.
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

/// POST /api/v1/templates/:name/render — Render a template with variables.
pub async fn render_template(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(req): Json<RenderTemplateRequest>,
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

/// PUT /api/v1/templates/:name — Update an existing template.
pub async fn update_template(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(req): Json<UpdateTemplateRequest>,
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

/// DELETE /api/v1/templates/:name — Delete a template.
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
