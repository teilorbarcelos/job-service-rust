use crate::{
    core::query_parser::{PaginatedResponse, QueryValidator},
    errors::AppError,
    infra::cache::Cache,
    modules::audit::schemas::AuditLogResponse,
    modules::audit::service::AuditModuleService,
};
use axum::{
    extract::{Query, State},
    Json,
};
use sea_orm::DatabaseConnection;

#[utoipa::path(
    get,
    path = "/v1/audit/all",
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
        (status = 200, description = "List of audit logs retrieved successfully", body = PaginatedAuditLogResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearerAuth" = [])
    ),
    tag = "Audit"
)]
pub async fn list_audit_logs_handler(
    State(state): State<(DatabaseConnection, Cache, crate::config::AppConfig)>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<PaginatedResponse<AuditLogResponse>>, AppError> {
    let (db, _, _) = state;
    let parsed_filters = QueryValidator::validate_and_parse(
        &params,
        &["username"],
        &["username", "createdAt", "updatedAt"],
    )?;

    let logs = AuditModuleService::list_audit_logs(parsed_filters, &db).await?;
    Ok(Json(logs))
}
