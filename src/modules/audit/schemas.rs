use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct AuditLogResponse {
    pub id: String,
    pub id_user: String,
    pub user_name: String,
    pub action_type: String,
    pub execute_type: String,
    pub class: String,
    pub function: String,
    pub params: String,
    pub raw: String,
    pub table_name: String,
    pub diff_value: String,
    pub original_url: String,
    pub method: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PaginatedAuditLogResponse {
    pub items: Vec<AuditLogResponse>,
    pub total: u64,
    pub page: u64,
    pub size: u64,
}
