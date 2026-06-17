use crate::errors::AppError;
use crate::infra::cache::Cache;
use crate::infra::storage::StorageProvider;
use axum::{extract::State, Json};
use sea_orm::DatabaseConnection;

#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct UploadResponse {
    pub url: String,
}

#[utoipa::path(
    post,
    path = "/v1/upload",
    request_body(content = String, description = "Multipart file upload", content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "File uploaded successfully", body = UploadResponse),
        (status = 400, description = "Invalid request data"),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearerAuth" = [])
    ),
    tag = "Upload"
)]
pub async fn upload_file_handler(
    State(_state): State<(DatabaseConnection, Cache, crate::config::AppConfig)>,
    mut multipart: axum::extract::Multipart,
) -> Result<Json<UploadResponse>, AppError> {
    if let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("Failed to parse multipart field: {}", e)))?
    {
        let file_name = field.file_name().unwrap_or("unnamed_file").to_string();

        let data = field
            .bytes()
            .await
            .map_err(|e| AppError::BadRequest(format!("Failed to read multipart data: {}", e)))?;

        if data.is_empty() {
            return Err(AppError::BadRequest("File is empty".to_string()));
        }

        let storage = StorageProvider::get();
        let url = storage.upload(&file_name, &data).await?;

        return Ok(Json(UploadResponse { url }));
    }

    Err(AppError::BadRequest("No file found in request".to_string()))
}
