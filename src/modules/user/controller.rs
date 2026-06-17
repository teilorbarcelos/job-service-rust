use crate::{
    core::query_parser::PaginatedResponse,
    errors::{AppError, AppJson},
    infra::cache::Cache,
    models::user,
    modules::user::schemas::{CreateUserRequest, UpdateUserRequest, UserResponse},
    modules::user::service::UserModuleService,
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use sea_orm::DatabaseConnection;

#[utoipa::path(
    get,
    path = "/v1/user",
    params(
        ("page" = Option<u64>, Query, description = "Page number"),
        ("size" = Option<u64>, Query, description = "Page size"),
        ("searchWord" = Option<String>, Query, description = "Search query word"),
        ("searchFields" = Option<String>, Query, description = "Comma-separated fields to search in"),
        ("orderBy" = Option<String>, Query, description = "Field to order by"),
        ("orderDirection" = Option<String>, Query, description = "Order direction (asc/desc)"),
        ("active" = Option<bool>, Query, description = "Filter by active status"),
    ),
    responses(
        (status = 200, description = "List of users retrieved successfully", body = PaginatedUserResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearerAuth" = [])
    ),
    tag = "User"
)]
pub async fn list_users_handler(
    State(state): State<(DatabaseConnection, Cache, crate::config::AppConfig)>,
    uri: axum::http::Uri,
    Query(mut params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<PaginatedResponse<UserResponse>>, AppError> {
    let (db, _, _) = state;
    if uri.path().ends_with("/all") {
        params.insert("ignoreDefaultFilters".to_string(), "true".to_string());
    }

    let parsed_filters = crate::core::crud::validate_and_parse::<user::Entity>(&params)?;

    let users = UserModuleService::list_users(parsed_filters, &db).await?;
    Ok(Json(users))
}

#[utoipa::path(
    get,
    path = "/v1/user/{id}",
    params(
        ("id" = String, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "User profile retrieved successfully", body = UserResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "User not found")
    ),
    security(
        ("bearerAuth" = [])
    ),
    tag = "User"
)]
pub async fn get_user_handler(
    State(state): State<(DatabaseConnection, Cache, crate::config::AppConfig)>,
    Path(id): Path<String>,
) -> Result<Json<UserResponse>, AppError> {
    let (db, _, _) = state;
    let user = UserModuleService::get_user_by_id(&id, &db).await?;
    Ok(Json(user))
}

#[utoipa::path(
    post,
    path = "/v1/user",
    request_body = CreateUserRequest,
    responses(
        (status = 201, description = "User created successfully", body = UserResponse),
        (status = 400, description = "Invalid request data"),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearerAuth" = [])
    ),
    tag = "User"
)]
pub async fn create_user_handler(
    State(state): State<(DatabaseConnection, Cache, crate::config::AppConfig)>,
    AppJson(payload): AppJson<CreateUserRequest>,
) -> Result<impl IntoResponse, AppError> {
    let (db, _, _) = state;
    let created = UserModuleService::create_user(payload, &db).await?;
    Ok((StatusCode::CREATED, Json(created)))
}

#[utoipa::path(
    put,
    path = "/v1/user/{id}",
    params(
        ("id" = String, Path, description = "User ID")
    ),
    request_body = UpdateUserRequest,
    responses(
        (status = 200, description = "User updated successfully", body = UserResponse),
        (status = 400, description = "Invalid request data"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "User not found")
    ),
    security(
        ("bearerAuth" = [])
    ),
    tag = "User"
)]
pub async fn update_user_handler(
    State(state): State<(DatabaseConnection, Cache, crate::config::AppConfig)>,
    Path(id): Path<String>,
    AppJson(payload): AppJson<UpdateUserRequest>,
) -> Result<Json<UserResponse>, AppError> {
    let (db, cache, _) = state;
    let updated = UserModuleService::update_user(&id, payload, &db, &cache).await?;
    Ok(Json(updated))
}

#[utoipa::path(
    delete,
    path = "/v1/user/{id}",
    params(
        ("id" = String, Path, description = "User ID")
    ),
    responses(
        (status = 204, description = "User deleted successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "User not found")
    ),
    security(
        ("bearerAuth" = [])
    ),
    tag = "User"
)]
pub async fn delete_user_handler(
    State(state): State<(DatabaseConnection, Cache, crate::config::AppConfig)>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let (db, cache, _) = state;
    UserModuleService::delete_user(&id, &db, &cache).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct ToggleStatusRequest {
    pub active: bool,
}

#[utoipa::path(
    patch,
    path = "/v1/user/{id}/status",
    params(
        ("id" = String, Path, description = "User ID")
    ),
    request_body = ToggleStatusRequest,
    responses(
        (status = 200, description = "User status toggled successfully", body = UserResponse),
        (status = 400, description = "Invalid request data"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "User not found")
    ),
    security(
        ("bearerAuth" = [])
    ),
    tag = "User"
)]
pub async fn toggle_user_status_handler(
    State(state): State<(DatabaseConnection, Cache, crate::config::AppConfig)>,
    Path(id): Path<String>,
    AppJson(payload): AppJson<ToggleStatusRequest>,
) -> Result<Json<UserResponse>, AppError> {
    let (db, cache, _) = state;

    let updated = UserModuleService::toggle_user_status(&id, payload.active, &db, &cache).await?;
    Ok(Json(updated))
}

#[utoipa::path(
    get,
    path = "/v1/user/export/pdf",
    params(
        ("searchWord" = Option<String>, Query, description = "Search query word"),
        ("searchFields" = Option<String>, Query, description = "Comma-separated fields to search in"),
        ("orderBy" = Option<String>, Query, description = "Field to order by"),
        ("orderDirection" = Option<String>, Query, description = "Order direction (asc/desc)"),
        ("active" = Option<bool>, Query, description = "Filter by active status"),
    ),
    responses(
        (status = 200, description = "PDF report retrieved successfully", body = Vec<u8>, content_type = "application/pdf"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(
        ("bearerAuth" = [])
    ),
    tag = "User"
)]
pub async fn export_pdf_handler(
    State(state): State<(DatabaseConnection, Cache, crate::config::AppConfig)>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<impl IntoResponse, AppError> {
    let (db, _, config) = state;

    let parsed_filters = crate::core::crud::validate_and_parse::<user::Entity>(&params)?;

    let pdf_bytes =
        UserModuleService::export_users_pdf(parsed_filters, &config.pdf_service_url, &db).await?;

    let response = axum::response::Response::builder()
        .header("Content-Type", "application/pdf")
        .header(
            "Content-Disposition",
            "attachment; filename=\"usuarios.pdf\"",
        )
        .body(axum::body::Body::from(pdf_bytes))
        .map_err(|e| AppError::Internal(format!("Failed to build response: {}", e)))?;

    Ok(response)
}
