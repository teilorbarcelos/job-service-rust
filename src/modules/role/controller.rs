use crate::{
    core::query_parser::PaginatedResponse,
    errors::{AppError, AppJson},
    infra::cache::Cache,
    models::role,
    modules::role::schemas::{CreateRoleRequest, FeatureResponse, RoleResponse, UpdateRoleRequest},
    modules::role::service::RoleModuleService,
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
    path = "/v1/role",
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
        (status = 200, description = "List of roles retrieved successfully", body = PaginatedRoleResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearerAuth" = [])
    ),
    tag = "Role"
)]
pub async fn list_roles_handler(
    State(state): State<(DatabaseConnection, Cache, crate::config::AppConfig)>,
    uri: axum::http::Uri,
    Query(mut params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<PaginatedResponse<RoleResponse>>, AppError> {
    let (db, _, _) = state;
    if uri.path().ends_with("/all") {
        params.insert("ignoreDefaultFilters".to_string(), "true".to_string());
    }

    let parsed_filters = crate::core::crud::validate_and_parse::<role::Entity>(&params)?;

    let roles = RoleModuleService::list_roles(parsed_filters, &db).await?;
    Ok(Json(roles))
}

#[utoipa::path(
    get,
    path = "/v1/role/{id}",
    params(
        ("id" = String, Path, description = "Role ID")
    ),
    responses(
        (status = 200, description = "Role retrieved successfully", body = RoleResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Role not found")
    ),
    security(
        ("bearerAuth" = [])
    ),
    tag = "Role"
)]
pub async fn get_role_handler(
    State(state): State<(DatabaseConnection, Cache, crate::config::AppConfig)>,
    Path(id): Path<String>,
) -> Result<Json<RoleResponse>, AppError> {
    let (db, _, _) = state;
    let role = RoleModuleService::get_role_by_id(&id, &db).await?;
    Ok(Json(role))
}

#[utoipa::path(
    post,
    path = "/v1/role",
    request_body = CreateRoleRequest,
    responses(
        (status = 201, description = "Role created successfully", body = RoleResponse),
        (status = 400, description = "Invalid request data"),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearerAuth" = [])
    ),
    tag = "Role"
)]
pub async fn create_role_handler(
    State(state): State<(DatabaseConnection, Cache, crate::config::AppConfig)>,
    AppJson(payload): AppJson<CreateRoleRequest>,
) -> Result<impl IntoResponse, AppError> {
    let (db, _, _) = state;
    let created = RoleModuleService::create_role(payload, &db).await?;
    Ok((StatusCode::CREATED, Json(created)))
}

#[utoipa::path(
    put,
    path = "/v1/role/{id}",
    params(
        ("id" = String, Path, description = "Role ID")
    ),
    request_body = UpdateRoleRequest,
    responses(
        (status = 200, description = "Role updated successfully", body = RoleResponse),
        (status = 400, description = "Invalid request data"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Role not found")
    ),
    security(
        ("bearerAuth" = [])
    ),
    tag = "Role"
)]
pub async fn update_role_handler(
    State(state): State<(DatabaseConnection, Cache, crate::config::AppConfig)>,
    Path(id): Path<String>,
    AppJson(payload): AppJson<UpdateRoleRequest>,
) -> Result<Json<RoleResponse>, AppError> {
    let (db, cache, _) = state;
    let updated = RoleModuleService::update_role(&id, payload, &db, &cache).await?;
    Ok(Json(updated))
}

#[utoipa::path(
    delete,
    path = "/v1/role/{id}",
    params(
        ("id" = String, Path, description = "Role ID")
    ),
    responses(
        (status = 204, description = "Role deleted successfully"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Role not found")
    ),
    security(
        ("bearerAuth" = [])
    ),
    tag = "Role"
)]
pub async fn delete_role_handler(
    State(state): State<(DatabaseConnection, Cache, crate::config::AppConfig)>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let (db, cache, _) = state;
    RoleModuleService::delete_role(&id, &db, &cache).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct ToggleStatusRequest {
    pub active: bool,
}

#[utoipa::path(
    patch,
    path = "/v1/role/{id}/status",
    params(
        ("id" = String, Path, description = "Role ID")
    ),
    request_body = ToggleStatusRequest,
    responses(
        (status = 200, description = "Role status toggled successfully", body = RoleResponse),
        (status = 400, description = "Invalid request data"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Role not found")
    ),
    security(
        ("bearerAuth" = [])
    ),
    tag = "Role"
)]
pub async fn toggle_role_status_handler(
    State(state): State<(DatabaseConnection, Cache, crate::config::AppConfig)>,
    Path(id): Path<String>,
    AppJson(payload): AppJson<ToggleStatusRequest>,
) -> Result<Json<RoleResponse>, AppError> {
    let (db, cache, _) = state;

    let updated = RoleModuleService::toggle_role_status(&id, payload.active, &db, &cache).await?;
    Ok(Json(updated))
}

#[utoipa::path(
    get,
    path = "/v1/role/features",
    responses(
        (status = 200, description = "List of system features retrieved successfully", body = [FeatureResponse]),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearerAuth" = [])
    ),
    tag = "Role"
)]
pub async fn list_features_handler(
    State(state): State<(DatabaseConnection, Cache, crate::config::AppConfig)>,
) -> Result<Json<Vec<FeatureResponse>>, AppError> {
    let (db, _, _) = state;

    let features = RoleModuleService::list_features(&db).await?;
    Ok(Json(features))
}
