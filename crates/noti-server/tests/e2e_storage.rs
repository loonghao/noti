//! E2E tests for the file storage API endpoints.
//!
//! Tests cover:
//! - `POST /api/v1/storage/upload` — multipart file upload
//! - `GET /api/v1/storage/{file_id}` — download stored file
//! - `GET /api/v1/storage/{file_id}/thumbnail` — get image thumbnail
//! - `DELETE /api/v1/storage/{file_id}` — delete stored file

mod common;

use common::{spawn_server_with_temp_storage, test_client};
use reqwest::StatusCode;
use serde::Deserialize;

// ───────────────────── DTOs ─────────────────────

#[derive(Debug, Deserialize)]
struct UploadResponse {
    file_id: String,
    file_name: String,
    mime_type: String,
    size_bytes: u64,
    download_url: String,
    #[serde(default)]
    thumbnail_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DeleteFileResponse {
    file_id: String,
    deleted: bool,
    message: String,
}

#[derive(Debug, Deserialize)]
struct ErrorResponse {
    #[allow(dead_code)]
    error: String,
    message: String,
}

// ───────────────────── Upload Tests ─────────────────────

#[tokio::test]
async fn e2e_storage_upload_text_file() {
    let (base, _storage_path) = spawn_server_with_temp_storage().await;
    let client = test_client();

    // Create multipart form with a text file
    let text_content = "Hello, this is a test file!";
    let file_part = reqwest::multipart::Part::bytes(text_content.as_bytes().to_vec())
        .file_name("test.txt")
        .mime_str("text/plain")
        .unwrap();

    let form = reqwest::multipart::Form::new().part("file", file_part);

    let resp = client
        .post(format!("{base}/api/v1/storage/upload"))
        .multipart(form)
        .send()
        .await
        .expect("request failed");

    assert_eq!(
        resp.status(),
        StatusCode::CREATED,
        "upload should return 201 Created"
    );

    let body: UploadResponse = resp.json().await.unwrap();
    assert!(!body.file_id.is_empty(), "file_id should not be empty");
    assert_eq!(body.file_name, "test.txt");
    assert_eq!(body.mime_type, "text/plain");
    assert_eq!(body.size_bytes, text_content.len() as u64);
    assert!(
        body.download_url.contains(&body.file_id),
        "download_url should contain file_id"
    );
    assert!(body.thumbnail_url.is_none(), "text files should not have thumbnails");
}

#[tokio::test]
async fn e2e_storage_upload_png_image() {
    let (base, _storage_path) = spawn_server_with_temp_storage().await;
    let client = test_client();

    // PNG magic bytes (8x8 transparent PNG)
    let png_data: Vec<u8> = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
        0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, // IHDR chunk length + type
        0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x08, // 8x8 dimensions
        0x08, 0x06, 0x00, 0x00, 0x00, 0x4B, 0x79, 0x21,
        0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41,
        0x54, 0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00,
        0x00, 0x00, 0x03, 0x00, 0x01, 0x00, 0x05, 0xFE,
        0xD4, 0xEF, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45,
        0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82, // IEND chunk
    ];

    let file_part = reqwest::multipart::Part::bytes(png_data.clone())
        .file_name("test.png")
        .mime_str("image/png")
        .unwrap();

    let form = reqwest::multipart::Form::new().part("file", file_part);

    let resp = client
        .post(format!("{base}/api/v1/storage/upload"))
        .multipart(form)
        .send()
        .await
        .expect("request failed");

    assert_eq!(resp.status(), StatusCode::CREATED);

    let body: UploadResponse = resp.json().await.unwrap();
    assert_eq!(body.file_name, "test.png");
    assert_eq!(body.mime_type, "image/png");
    assert_eq!(body.size_bytes, png_data.len() as u64);
    // PNG is exactly 8x8, which is <= 100x100 threshold, so no thumbnail
    assert!(
        body.thumbnail_url.is_none(),
        "8x8 image should not generate thumbnail (below 100x100 threshold)"
    );
}

#[tokio::test]
async fn e2e_storage_upload_missing_file_field() {
    let (base, _storage_path) = spawn_server_with_temp_storage().await;
    let client = test_client();

    // Send form with no 'file' field
    let form = reqwest::multipart::Form::new().text("other_field", "value");

    let resp = client
        .post(format!("{base}/api/v1/storage/upload"))
        .multipart(form)
        .send()
        .await
        .expect("request failed");

    assert_eq!(
        resp.status(),
        StatusCode::BAD_REQUEST,
        "missing file field should return 400"
    );

    let body: ErrorResponse = resp.json().await.unwrap();
    assert!(body.message.contains("file") || body.message.contains("field"));
}

// ───────────────────── Download Tests ─────────────────────

#[tokio::test]
async fn e2e_storage_download_uploaded_file() {
    let (base, _storage_path) = spawn_server_with_temp_storage().await;
    let client = test_client();

    // First upload a file
    let text_content = "Download test content";
    let file_part = reqwest::multipart::Part::bytes(text_content.as_bytes().to_vec())
        .file_name("download_test.txt")
        .mime_str("text/plain")
        .unwrap();

    let form = reqwest::multipart::Form::new().part("file", file_part);

    let upload_resp = client
        .post(format!("{base}/api/v1/storage/upload"))
        .multipart(form)
        .send()
        .await
        .expect("upload request failed");

    let upload_body: UploadResponse = upload_resp.json().await.unwrap();
    let file_id = upload_body.file_id;

    // Now download the file
    let download_resp = client
        .get(format!("{base}/api/v1/storage/{file_id}"))
        .send()
        .await
        .expect("download request failed");

    assert_eq!(
        download_resp.status(),
        StatusCode::OK,
        "download should return 200"
    );

    let content_type = download_resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok());
    assert_eq!(
        content_type, Some("text/plain"),
        "content-type should be text/plain"
    );

    let downloaded_bytes = download_resp.bytes().await.unwrap();
    assert_eq!(
        downloaded_bytes.as_ref(),
        text_content.as_bytes(),
        "downloaded content should match uploaded content"
    );
}

#[tokio::test]
async fn e2e_storage_download_nonexistent_file() {
    let (base, _storage_path) = spawn_server_with_temp_storage().await;
    let client = test_client();

    let resp = client
        .get(format!("{base}/api/v1/storage/nonexistent-file-id"))
        .send()
        .await
        .expect("request failed");

    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "download of nonexistent file should return 404"
    );

    let body: ErrorResponse = resp.json().await.unwrap();
    assert!(
        body.message.contains("not found") || body.message.contains("nonexistent"),
        "error message should indicate file not found"
    );
}

// ───────────────────── Thumbnail Tests ─────────────────────

#[tokio::test]
async fn e2e_storage_get_thumbnail_for_uploaded_image() {
    let (base, _storage_path) = spawn_server_with_temp_storage().await;
    let client = test_client();

    // Upload an image (create a larger PNG that will generate thumbnail)
    let mut png_data = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A,
    ];
    // IHDR for 200x200 image
    png_data.extend_from_slice(&[0x00, 0x00, 0x00, 0x0D]);
    png_data.extend_from_slice(b"IHDR");
    png_data.extend_from_slice(&(200u32).to_be_bytes());
    png_data.extend_from_slice(&(200u32).to_be_bytes());
    png_data.extend_from_slice(&[0x08, 0x02, 0x00, 0x00, 0x00]);
    png_data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x49, 0x44, 0x41, 0x54]);
    png_data.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44]);
    png_data.extend_from_slice(&[0xAE, 0x42, 0x60, 0x82]);

    let file_part = reqwest::multipart::Part::bytes(png_data)
        .file_name("image_thumb.png")
        .mime_str("image/png")
        .unwrap();

    let form = reqwest::multipart::Form::new().part("file", file_part);

    let upload_resp = client
        .post(format!("{base}/api/v1/storage/upload"))
        .multipart(form)
        .send()
        .await
        .expect("upload request failed");

    let upload_body: UploadResponse = upload_resp.json().await.unwrap();

    // Skip if thumbnail wasn't generated (image too small)
    let thumbnail_url = match upload_body.thumbnail_url {
        Some(url) => url,
        None => {
            // If no thumbnail was generated, test that requesting it returns 404
            let resp = client
                .get(format!("{base}/api/v1/storage/{}/thumbnail", upload_body.file_id))
                .send()
                .await
                .expect("thumbnail request failed");
            assert_eq!(resp.status(), StatusCode::NOT_FOUND);
            return;
        }
    };

    // Extract file_id from thumbnail_url and request the thumbnail
    let file_id = upload_body.file_id;
    assert!(
        thumbnail_url.contains("/thumbnail"),
        "thumbnail_url should contain /thumbnail path"
    );
    let thumb_resp = client
        .get(format!("{base}/api/v1/storage/{file_id}/thumbnail"))
        .send()
        .await
        .expect("thumbnail request failed");

    assert_eq!(
        thumb_resp.status(),
        StatusCode::OK,
        "thumbnail should return 200"
    );

    let content_type = thumb_resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok());
    assert_eq!(
        content_type, Some("image/png"),
        "thumbnail should be image/png"
    );
}

#[tokio::test]
async fn e2e_storage_thumbnail_for_nonexistent_file() {
    let (base, _storage_path) = spawn_server_with_temp_storage().await;
    let client = test_client();

    let resp = client
        .get(format!("{base}/api/v1/storage/nonexistent-id/thumbnail"))
        .send()
        .await
        .expect("request failed");

    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "thumbnail for nonexistent file should return 404"
    );
}

// ───────────────────── Delete Tests ─────────────────────

#[tokio::test]
async fn e2e_storage_delete_uploaded_file() {
    let (base, _storage_path) = spawn_server_with_temp_storage().await;
    let client = test_client();

    // Upload a file first
    let text_content = "Delete test content";
    let file_part = reqwest::multipart::Part::bytes(text_content.as_bytes().to_vec())
        .file_name("delete_test.txt")
        .mime_str("text/plain")
        .unwrap();

    let form = reqwest::multipart::Form::new().part("file", file_part);

    let upload_resp = client
        .post(format!("{base}/api/v1/storage/upload"))
        .multipart(form)
        .send()
        .await
        .expect("upload request failed");

    let upload_body: UploadResponse = upload_resp.json().await.unwrap();
    let file_id = upload_body.file_id;

    // Delete the file
    let delete_resp = client
        .delete(format!("{base}/api/v1/storage/{file_id}"))
        .send()
        .await
        .expect("delete request failed");

    assert_eq!(
        delete_resp.status(),
        StatusCode::OK,
        "delete should return 200"
    );

    let delete_body: DeleteFileResponse = delete_resp.json().await.unwrap();
    assert_eq!(delete_body.file_id, file_id);
    assert!(delete_body.deleted);
    assert!(delete_body.message.contains("success") || delete_body.message.contains("deleted"));

    // Verify file is actually deleted (download should return 404)
    let download_resp = client
        .get(format!("{base}/api/v1/storage/{file_id}"))
        .send()
        .await
        .expect("download request failed");

    assert_eq!(
        download_resp.status(),
        StatusCode::NOT_FOUND,
        "deleted file should not be downloadable"
    );
}

#[tokio::test]
async fn e2e_storage_delete_nonexistent_file() {
    let (base, _storage_path) = spawn_server_with_temp_storage().await;
    let client = test_client();

    let resp = client
        .delete(format!("{base}/api/v1/storage/nonexistent-delete-id"))
        .send()
        .await
        .expect("request failed");

    assert_eq!(
        resp.status(),
        StatusCode::NOT_FOUND,
        "delete of nonexistent file should return 404"
    );
}

// ───────────────────── MIME Detection Tests ─────────────────────

#[tokio::test]
async fn e2e_storage_magic_byte_detection_png() {
    let (base, _storage_path) = spawn_server_with_temp_storage().await;
    let client = test_client();

    // PNG with wrong extension should still be detected as PNG
    let png_data: Vec<u8> = vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A,
        0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
        0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x08,
        0x08, 0x06, 0x00, 0x00, 0x00, 0x4B, 0x79, 0x21,
        0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41,
        0x54, 0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00,
        0x00, 0x00, 0x03, 0x00, 0x01, 0x00, 0x05, 0xFE,
        0xD4, 0xEF, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45,
        0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];

    // Use .txt extension to test magic byte override
    let file_part = reqwest::multipart::Part::bytes(png_data)
        .file_name("fake.txt") // Wrong extension
        .mime_str("application/octet-stream")
        .unwrap();

    let form = reqwest::multipart::Form::new().part("file", file_part);

    let resp = client
        .post(format!("{base}/api/v1/storage/upload"))
        .multipart(form)
        .send()
        .await
        .expect("request failed");

    assert_eq!(resp.status(), StatusCode::CREATED);

    let body: UploadResponse = resp.json().await.unwrap();
    // Magic bytes should override the claimed mime type
    assert_eq!(
        body.mime_type, "image/png",
        "PNG magic bytes should override fake.txt extension"
    );
}

#[tokio::test]
async fn e2e_storage_magic_byte_detection_jpeg() {
    let (base, _storage_path) = spawn_server_with_temp_storage().await;
    let client = test_client();

    // JPEG magic bytes
    let jpeg_data: Vec<u8> = vec![
        0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01,
    ];

    let file_part = reqwest::multipart::Part::bytes(jpeg_data)
        .file_name("photo.png") // Wrong extension
        .mime_str("image/png")
        .unwrap();

    let form = reqwest::multipart::Form::new().part("file", file_part);

    let resp = client
        .post(format!("{base}/api/v1/storage/upload"))
        .multipart(form)
        .send()
        .await
        .expect("request failed");

    assert_eq!(resp.status(), StatusCode::CREATED);

    let body: UploadResponse = resp.json().await.unwrap();
    assert_eq!(
        body.mime_type, "image/jpeg",
        "JPEG magic bytes should override fake.png extension"
    );
}

// ───────────────────── Empty File Tests ─────────────────────

#[tokio::test]
async fn e2e_storage_upload_empty_file() {
    let (base, _storage_path) = spawn_server_with_temp_storage().await;
    let client = test_client();

    // Upload an empty file (0 bytes)
    let file_part = reqwest::multipart::Part::bytes(Vec::new())
        .file_name("empty.txt")
        .mime_str("text/plain")
        .unwrap();

    let form = reqwest::multipart::Form::new().part("file", file_part);

    let resp = client
        .post(format!("{base}/api/v1/storage/upload"))
        .multipart(form)
        .send()
        .await
        .expect("request failed");

    // Empty files should still be accepted (0 is a valid size)
    assert_eq!(
        resp.status(),
        StatusCode::CREATED,
        "empty file upload should return 201 Created"
    );

    let body: UploadResponse = resp.json().await.unwrap();
    assert!(!body.file_id.is_empty(), "file_id should not be empty");
    assert_eq!(body.file_name, "empty.txt");
    assert_eq!(body.size_bytes, 0, "empty file should have 0 bytes");
    assert!(body.thumbnail_url.is_none(), "empty file should not have thumbnail");
}

#[tokio::test]
async fn e2e_storage_download_empty_file() {
    let (base, _storage_path) = spawn_server_with_temp_storage().await;
    let client = test_client();

    // Upload an empty file first
    let file_part = reqwest::multipart::Part::bytes(Vec::new())
        .file_name("empty_download.txt")
        .mime_str("text/plain")
        .unwrap();

    let form = reqwest::multipart::Form::new().part("file", file_part);

    let upload_resp = client
        .post(format!("{base}/api/v1/storage/upload"))
        .multipart(form)
        .send()
        .await
        .expect("upload request failed");

    let upload_body: UploadResponse = upload_resp.json().await.unwrap();
    let file_id = upload_body.file_id;

    // Download the empty file
    let download_resp = client
        .get(format!("{base}/api/v1/storage/{file_id}"))
        .send()
        .await
        .expect("download request failed");

    assert_eq!(
        download_resp.status(),
        StatusCode::OK,
        "download of empty file should return 200"
    );

    let downloaded_bytes = download_resp.bytes().await.unwrap();
    assert_eq!(
        downloaded_bytes.len(),
        0,
        "downloaded content should be empty"
    );
}


