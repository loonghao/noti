//! File storage handlers: upload, download, and image thumbnail generation.
//!
//! Provides:
//! - `POST /api/v1/storage/upload` — multipart file upload
//! - `GET /api/v1/storage/{file_id}` — download stored file
//! - `GET /api/v1/storage/{file_id}/thumbnail` — get image thumbnail (images only)

use std::path::PathBuf;

use axum::{
    extract::{Multipart, Path, State},
    http::HeaderValue,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::handlers::error::ApiError;
use crate::state::AppState;

// ───────────────────── DTOs ─────────────────────

/// Response for a successful file upload.
#[derive(Debug, Serialize, ToSchema)]
pub struct UploadResponse {
    /// Unique file ID that can be used to reference this file.
    pub file_id: String,
    /// Original file name.
    pub file_name: String,
    /// MIME type of the uploaded file.
    pub mime_type: String,
    /// File size in bytes.
    pub size_bytes: u64,
    /// URL to download this file.
    pub download_url: String,
    /// URL to get the thumbnail (images only, null otherwise).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbnail_url: Option<String>,
}

/// Metadata about a stored file.
#[derive(Debug, Serialize, ToSchema)]
pub struct FileMetadata {
    pub file_id: String,
    pub file_name: String,
    pub mime_type: String,
    pub size_bytes: u64,
    pub is_image: bool,
    pub has_thumbnail: bool,
}

// ───────────────────── Storage Service ─────────────────────

/// Store a file on disk and return its metadata.
#[allow(clippy::ptr_arg)]
pub async fn store_file(
    storage_root: &PathBuf,
    data: Vec<u8>,
    original_name: &str,
    _mime_type: &str,
) -> Result<(String, PathBuf), std::io::Error> {
    let file_id = Uuid::new_v4().to_string();
    let ext = std::path::Path::new(original_name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("bin");
    // Store files in the uploads subdirectory to match where download looks
    let file_path = storage_root.join("uploads").join(format!("{}.{}", file_id, ext));

    let mut file = tokio::fs::File::create(&file_path).await?;
    file.write_all(&data).await?;

    Ok((file_id, file_path))
}

/// Generate a thumbnail for an image file.
/// Returns the path to the thumbnail on success, or None if thumbnail generation fails.
#[allow(clippy::ptr_arg)]
pub async fn generate_thumbnail(
    source_path: &PathBuf,
    thumbnail_dir: &PathBuf,
    file_id: &str,
) -> Result<Option<PathBuf>, std::io::Error> {
    // Check if it's actually an image by trying to decode it
    let (width, height) = match image::image_dimensions(source_path) {
        Ok(dims) => dims,
        Err(_) => return Ok(None),
    };

    // Only generate thumbnail if image is larger than 100x100
    if width <= 100 && height <= 100 {
        return Ok(None);
    }

    // Open the image for thumbnail generation
    let img = match image::open(source_path) {
        Ok(img) => img,
        Err(_) => return Ok(None),
    };

    // Create thumbnail: resize to max 200x200 maintaining aspect ratio
    let thumbnail = img.thumbnail(200, 200);
    let thumb_path = thumbnail_dir.join(format!("{}.png", file_id));

    thumbnail.save(&thumb_path).map_err(|e| {
        std::io::Error::other(format!("thumbnail save failed: {}", e))
    })?;
    Ok(Some(thumb_path))
}

/// Determine MIME type from file data (magic bytes) or extension.
fn detect_mime(data: &[u8], filename: &str) -> String {
    // Check common magic bytes
    if data.len() >= 8 {
        // PNG
        if &data[0..8] == b"\x89PNG\r\n\x1a\n" {
            return "image/png".to_string();
        }
        // JPEG
        if data[0] == 0xFF && data[1] == 0xD8 && data[2] == 0xFF {
            return "image/jpeg".to_string();
        }
        // GIF
        if &data[0..6] == b"GIF87a" || &data[0..6] == b"GIF89a" {
            return "image/gif".to_string();
        }
        // WebP
        if &data[0..4] == b"RIFF" && &data[8..12] == b"WEBP" {
            return "image/webp".to_string();
        }
    }

    // Fall back to extension-based detection
    mime_guess::from_path(filename)
        .first()
        .map(|m| m.to_string())
        .unwrap_or_else(|| "application/octet-stream".to_string())
}

/// Check if a MIME type represents an image.
fn is_image_mime(mime: &str) -> bool {
    mime.starts_with("image/")
}

// ───────────────────── Handlers ─────────────────────

/// Upload a file via multipart form data.
#[utoipa::path(
    post,
    path = "/api/v1/storage/upload",
    tag = "Storage",
    request_body(content = UploadRequest, content_type = "multipart/form-data"),
    responses(
        (status = 201, description = "File uploaded successfully", body = UploadResponse),
        (status = 400, description = "Invalid file or missing content", body = ApiError),
    )
)]
pub async fn upload_file(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, ApiError> {
    let field_name = "file";

    // Ensure upload and thumbnail directories exist
    fs::create_dir_all(state.storage_dir()).await.map_err(|e| {
        ApiError::internal(format!("failed to create upload directory: {}", e))
    })?;
    fs::create_dir_all(state.thumbnails_dir()).await.map_err(|e| {
        ApiError::internal(format!("failed to create thumbnail directory: {}", e))
    })?;

    // Collect all fields from multipart
    let mut file_data: Option<Vec<u8>> = None;
    let mut filename = String::new();

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        ApiError::bad_request(format!("failed to read multipart field: {}", e))
    })? {
        let name = field.name().unwrap_or("").to_string();
        if name == field_name {
            filename = field
                .file_name()
                .unwrap_or("uploaded_file")
                .to_string();
            file_data = Some(
                field
                    .bytes()
                    .await
                    .map_err(|e| ApiError::bad_request(format!("failed to read file data: {}", e)))?
                    .to_vec(),
            );
        }
    }

    let data = file_data.ok_or_else(|| {
        ApiError::bad_request(format!("missing '{}' field in multipart form", field_name))
    })?;

    let size_bytes = data.len() as u64;
    let mime_type = detect_mime(&data, &filename);

    // Store the file
    let (file_id, file_path) = store_file(
        &state.storage_root,
        data,
        &filename,
        &mime_type,
    )
    .await
    .map_err(|e| ApiError::internal(format!("failed to store file: {}", e)))?;

    // Generate thumbnail for images
    let thumbnail_path = if is_image_mime(&mime_type) {
        match generate_thumbnail(&file_path, &state.thumbnails_dir(), &file_id).await {
            Ok(Some(_)) => Some(format!("/api/v1/storage/{}/thumbnail", file_id)),
            Ok(None) => None,
            Err(e) => {
                tracing::warn!(file_id = %file_id, error = %e, "thumbnail generation failed, continuing without thumbnail");
                None
            }
        }
    } else {
        None
    };

    let base_url = format!("/api/v1/storage/{}", file_id);

    tracing::info!(
        file_id = %file_id,
        filename = %filename,
        size_bytes,
        mime_type = %mime_type,
        "file uploaded"
    );

    Ok((
        axum::http::StatusCode::CREATED,
        Json(UploadResponse {
            file_id,
            file_name: filename,
            mime_type,
            size_bytes,
            download_url: base_url.clone(),
            thumbnail_url: thumbnail_path,
        }),
    ))
}

/// Download a stored file by ID.
#[utoipa::path(
    get,
    path = "/api/v1/storage/{file_id}",
    tag = "Storage",
    params(
        ("file_id" = String, Path, description = "Unique file ID from upload response")
    ),
    responses(
        (status = 200, description = "File content"),
        (status = 404, description = "File not found", body = ApiError),
    )
)]
pub async fn download_file(
    State(state): State<AppState>,
    Path(file_id): Path<String>,
) -> Result<Response, ApiError> {
    let upload_dir = state.storage_dir();

    // Check if upload directory exists
    if !upload_dir.exists() {
        return Err(ApiError::not_found(format!(
            "file '{}' not found (no files have been uploaded)",
            file_id
        )));
    }

    // Find the file: iterate to find any file with matching ID prefix
    let mut matching_path: Option<PathBuf> = None;
    let mut entries = fs::read_dir(&upload_dir).await.map_err(|e| {
        ApiError::internal(format!("failed to read upload directory: {}", e))
    })?;

    while let Some(entry) = entries.next_entry().await.map_err(|e| {
        ApiError::internal(format!("failed to read upload directory entry: {}", e))
    })? {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with(&format!("{}.", file_id)) {
            matching_path = Some(entry.path());
            break;
        }
    }

    let path = matching_path.ok_or_else(|| {
        ApiError::not_found(format!("file '{}' not found", file_id))
    })?;

    let data = fs::read(&path).await.map_err(|e| {
        ApiError::internal(format!("failed to read file: {}", e))
    })?;

    let mime_type = detect_mime(&data, path.file_name().unwrap_or_default().to_str().unwrap_or("file"));

    let mut resp = Response::new(data.into());
    resp.headers_mut().insert(
        axum::http::header::CONTENT_TYPE,
        HeaderValue::from_str(&mime_type).unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
    );
    resp.headers_mut().insert(
        axum::http::header::CONTENT_DISPOSITION,
        HeaderValue::from_static("inline"),
    );

    Ok(resp)
}

/// Get a thumbnail for an image file.
#[utoipa::path(
    get,
    path = "/api/v1/storage/{file_id}/thumbnail",
    tag = "Storage",
    params(
        ("file_id" = String, Path, description = "Unique file ID of an image")
    ),
    responses(
        (status = 200, description = "PNG thumbnail image"),
        (status = 404, description = "File or thumbnail not found", body = ApiError),
    )
)]
pub async fn get_thumbnail(
    State(state): State<AppState>,
    Path(file_id): Path<String>,
) -> Result<Response, ApiError> {
    let thumb_path = state.thumbnails_dir().join(format!("{}.png", file_id));

    if !thumb_path.exists() {
        return Err(ApiError::not_found(format!(
            "thumbnail for '{}' not found (may not be an image or thumbnail generation failed)",
            file_id
        )));
    }

    let data = fs::read(&thumb_path).await.map_err(|e| {
        ApiError::internal(format!("failed to read thumbnail: {}", e))
    })?;

    let mut resp = Response::new(data.into());
    resp.headers_mut().insert(
        axum::http::header::CONTENT_TYPE,
        HeaderValue::from_static("image/png"),
    );

    Ok(resp)
}

/// Delete a stored file by ID.
#[utoipa::path(
    delete,
    path = "/api/v1/storage/{file_id}",
    tag = "Storage",
    params(
        ("file_id" = String, Path, description = "Unique file ID to delete")
    ),
    responses(
        (status = 200, description = "File deleted", body = DeleteFileResponse),
        (status = 404, description = "File not found", body = ApiError),
    )
)]
pub async fn delete_file(
    State(state): State<AppState>,
    Path(file_id): Path<String>,
) -> Result<Json<DeleteFileResponse>, ApiError> {
    let upload_dir = state.storage_dir();
    let thumb_dir = state.thumbnails_dir();

    // Check if upload directory exists
    if !upload_dir.exists() {
        return Err(ApiError::not_found(format!(
            "file '{}' not found (no files have been uploaded)",
            file_id
        )));
    }

    // Find and delete the uploaded file
    let mut deleted = false;
    let mut entries = fs::read_dir(&upload_dir).await.map_err(|e| {
        ApiError::internal(format!("failed to read upload directory: {}", e))
    })?;

    while let Some(entry) = entries.next_entry().await.map_err(|e| {
        ApiError::internal(format!("failed to read upload directory entry: {}", e))
    })? {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with(&format!("{}.", file_id)) {
            fs::remove_file(entry.path()).await.map_err(|e| {
                ApiError::internal(format!("failed to delete file: {}", e))
            })?;
            deleted = true;
            break;
        }
    }

    if !deleted {
        return Err(ApiError::not_found(format!("file '{}' not found", file_id)));
    }

    // Delete thumbnail if exists
    let thumb_path = thumb_dir.join(format!("{}.png", file_id));
    if thumb_path.exists() {
        let _ = fs::remove_file(&thumb_path).await;
    }

    tracing::info!(file_id = %file_id, "file deleted");

    Ok(Json(DeleteFileResponse {
        file_id,
        deleted: true,
        message: "File deleted successfully".to_string(),
    }))
}

/// Response for delete operation.
#[derive(Debug, Serialize, ToSchema)]
pub struct DeleteFileResponse {
    pub file_id: String,
    pub deleted: bool,
    pub message: String,
}

/// Request wrapper for multipart upload (multipart doesn't use JSON, but we define schema for OpenAPI).
#[derive(ToSchema)]
pub struct UploadRequest {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_mime_png() {
        let data = b"\x89PNG\r\n\x1a\n".to_vec();
        let mime = detect_mime(&data, "test.png");
        assert_eq!(mime, "image/png");
    }

    #[test]
    fn test_detect_mime_jpeg() {
        let data = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46];
        let mime = detect_mime(&data, "photo.jpg");
        assert_eq!(mime, "image/jpeg");
    }

    #[test]
    fn test_detect_mime_gif() {
        let data = b"GIF89a1".to_vec();
        let mime = detect_mime(&data, "animation.gif");
        assert_eq!(mime, "image/gif");
    }

    #[test]
    fn test_detect_mime_webp() {
        let data = b"RIFF\x00\x00\x00\x00WEBP";
        let mime = detect_mime(data, "image.webp");
        assert_eq!(mime, "image/webp");
    }

    #[test]
    fn test_detect_mime_fallback() {
        let data = b"PK\x03\x04".to_vec(); // ZIP magic
        let mime = detect_mime(&data, "document.pdf");
        // Falls back to extension-based
        assert_eq!(mime, "application/pdf");
    }

    #[test]
    fn test_is_image_mime() {
        assert!(is_image_mime("image/png"));
        assert!(is_image_mime("image/jpeg"));
        assert!(is_image_mime("image/gif"));
        assert!(!is_image_mime("text/plain"));
        assert!(!is_image_mime("application/pdf"));
        assert!(!is_image_mime("video/mp4"));
    }
}
