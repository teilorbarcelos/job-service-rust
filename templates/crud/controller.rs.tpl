use crate::{
    core::query_parser::PaginatedResponse,
    errors::{AppError, AppJson},
    infra::cache::Cache,
    models::{{entity_slug}},
    modules::{{entity_slug}}::schemas::{Create{{EntityName}}Request, {{EntityName}}Response, Update{{EntityName}}Request},
    modules::{{entity_slug}}::service::{{EntityName}}ModuleService,
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
    path = "/v1/{{entity_slug}}",
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
        (status = 200, description = "List retrieved successfully", body = Paginated{{EntityName}}Response),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearerAuth" = [])
    ),
    tag = "{{FeatureName}}"
)]
pub async fn list_{{entity_slug}}s_handler(
    State(state): State<(DatabaseConnection, Cache, crate::config::AppConfig)>,
    uri: axum::http::Uri,
    Query(mut params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<PaginatedResponse<{{EntityName}}Response>>, AppError> {
    let (db, _, _) = state;

    if uri.path().ends_with("/all") {
        params.insert("ignoreDefaultFilters".to_string(), "true".to_string());
    }

    let parsed_filters = crate::core::crud::validate_and_parse::<{{entity_slug}}::Entity>(&params)?;

    let items = {{EntityName}}ModuleService::list_{{entity_slug}}s(parsed_filters, &db).await?;
    Ok(Json(items))
}

#[utoipa::path(
    get,
    path = "/v1/{{entity_slug}}/{id}",
    params(
        ("id" = String, Path, description = "Record ID")
    ),
    responses(
        (status = 200, description = "Record retrieved successfully", body = {{EntityName}}Response),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Record not found")
    ),
    security(
        ("bearerAuth" = [])
    ),
    tag = "{{FeatureName}}"
)]
pub async fn get_{{entity_slug}}_handler(
    State(state): State<(DatabaseConnection, Cache, crate::config::AppConfig)>,
    Path(id): Path<String>,
) -> Result<Json<{{EntityName}}Response>, AppError> {
    let (db, _, _) = state;

    let item = {{EntityName}}ModuleService::get_{{entity_slug}}_by_id(&id, &db).await?;
    Ok(Json(item))
}

#[utoipa::path(
    post,
    path = "/v1/{{entity_slug}}",
    request_body = Create{{EntityName}}Request,
    responses(
        (status = 201, description = "Record created successfully", body = {{EntityName}}Response),
        (status = 400, description = "Invalid request data"),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearerAuth" = [])
    ),
    tag = "{{FeatureName}}"
)]
pub async fn create_{{entity_slug}}_handler(
    State(state): State<(DatabaseConnection, Cache, crate::config::AppConfig)>,
    AppJson(payload): AppJson<Create{{EntityName}}Request>,
) -> Result<impl IntoResponse, AppError> {
    let (db, _, _) = state;

    let created = {{EntityName}}ModuleService::create_{{entity_slug}}(payload, &db).await?;
    Ok((StatusCode::CREATED, Json(created)))
}

#[utoipa::path(
    put,
    path = "/v1/{{entity_slug}}/{id}",
    params(
        ("id" = String, Path, description = "Record ID")
    ),
    request_body = Update{{EntityName}}Request,
    responses(
        (status = 200, description = "Record updated successfully", body = {{EntityName}}Response),
        (status = 400, description = "Invalid request data"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Record not found")
    ),
    security(
        ("bearerAuth" = [])
    ),
    tag = "{{FeatureName}}"
)]
pub async fn update_{{entity_slug}}_handler(
    State(state): State<(DatabaseConnection, Cache, crate::config::AppConfig)>,
    Path(id): Path<String>,
    AppJson(payload): AppJson<Update{{EntityName}}Request>,
) -> Result<Json<{{EntityName}}Response>, AppError> {
    let (db, _, _) = state;

    let updated = {{EntityName}}ModuleService::update_{{entity_slug}}(&id, payload, &db).await?;
    Ok(Json(updated))
}

#[utoipa::path(
    delete,
    path = "/v1/{{entity_slug}}/{id}",
    params(
        ("id" = String, Path, description = "Record ID")
    ),
    responses(
        (status = 204, description = "Record deleted successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Record not found")
    ),
    security(
        ("bearerAuth" = [])
    ),
    tag = "{{FeatureName}}"
)]
pub async fn delete_{{entity_slug}}_handler(
    State(state): State<(DatabaseConnection, Cache, crate::config::AppConfig)>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let (db, _, _) = state;

    {{EntityName}}ModuleService::delete_{{entity_slug}}(&id, &db).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct ToggleStatusRequest {
    pub active: bool,
}

#[utoipa::path(
    patch,
    path = "/v1/{{entity_slug}}/{id}/status",
    params(
        ("id" = String, Path, description = "Record ID")
    ),
    request_body = ToggleStatusRequest,
    responses(
        (status = 200, description = "Record status toggled successfully", body = {{EntityName}}Response),
        (status = 400, description = "Invalid request data"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Record not found")
    ),
    security(
        ("bearerAuth" = [])
    ),
    tag = "{{FeatureName}}"
)]
pub async fn toggle_{{entity_slug}}_status_handler(
    State(state): State<(DatabaseConnection, Cache, crate::config::AppConfig)>,
    Path(id): Path<String>,
    AppJson(payload): AppJson<ToggleStatusRequest>,
) -> Result<Json<{{EntityName}}Response>, AppError> {
    let (db, _, _) = state;

    let updated = {{EntityName}}ModuleService::toggle_{{entity_slug}}_status(&id, payload.active, &db).await?;
    Ok(Json(updated))
}
