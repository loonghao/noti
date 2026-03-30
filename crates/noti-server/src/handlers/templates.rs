use std::collections::HashMap;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};

use noti_core::MessageTemplate;

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

/// Template list response.
#[derive(Debug, Serialize)]
pub struct TemplateListResponse {
    pub templates: Vec<String>,
    pub total: usize,
}

/// POST /api/v1/templates — Create a new message template.
pub async fn create_template(
    State(state): State<AppState>,
    Json(req): Json<CreateTemplateRequest>,
) -> Result<(StatusCode, Json<TemplateResponse>), (StatusCode, Json<serde_json::Value>)> {
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
) -> Result<Json<TemplateResponse>, (StatusCode, Json<serde_json::Value>)> {
    let registry = state.template_registry.read().await;
    let template = registry.get(&name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": format!("template '{}' not found", name)
            })),
        )
    })?;

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
) -> Result<Json<RenderedTemplateResponse>, (StatusCode, Json<serde_json::Value>)> {
    let registry = state.template_registry.read().await;
    let template = registry.get(&name).ok_or_else(|| {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": format!("template '{}' not found", name)
            })),
        )
    })?;

    if let Err(e) = template.validate_vars(&req.variables) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": e.to_string() })),
        ));
    }

    let text = template.render_body(&req.variables);
    let title = template.render_title(&req.variables);

    Ok(Json(RenderedTemplateResponse { text, title }))
}
