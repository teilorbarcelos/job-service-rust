use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Deserialize, Serialize, Clone, ToSchema)]
pub struct PermissionRequest {
    #[serde(rename = "id_feature")]
    pub feature: String,
    pub create: bool,
    pub view: bool,
    pub activate: bool,
    pub delete: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateRoleRequest {
    pub name: String,
    pub description: String,
    pub permissions: Vec<PermissionRequest>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateRoleRequest {
    pub name: String,
    pub description: String,
    pub permissions: Option<Vec<PermissionRequest>>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RoleResponse {
    pub id: String,
    pub name: String,
    pub description: String,
    pub active: bool,
    #[serde(rename = "RoleFeature")]
    pub role_feature: Vec<PermissionRequest>,
    pub created_at: String,
    pub updated_at: String,
    pub is_deleted: bool,
    pub deleted_at: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FeatureResponse {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PaginatedRoleResponse {
    pub items: Vec<RoleResponse>,
    pub total: u64,
    pub page: u64,
    pub size: u64,
}

impl From<crate::models::role_feature::Model> for PermissionRequest {
    fn from(p: crate::models::role_feature::Model) -> Self {
        Self {
            feature: p.id_feature,
            create: p.create,
            view: p.view,
            activate: p.activate,
            delete: p.delete,
        }
    }
}

impl From<crate::models::feature::Model> for FeatureResponse {
    fn from(f: crate::models::feature::Model) -> Self {
        Self {
            id: f.id,
            name: f.name,
        }
    }
}

impl From<(crate::models::role::Model, Vec<PermissionRequest>)> for RoleResponse {
    fn from((r, role_feature): (crate::models::role::Model, Vec<PermissionRequest>)) -> Self {
        Self {
            id: r.id,
            name: r.name,
            description: r.description,
            active: r.active,
            role_feature,
            created_at: r.created_at.to_rfc3339(),
            updated_at: r.updated_at.to_rfc3339(),
            is_deleted: r.is_deleted.unwrap_or(false),
            deleted_at: r.deleted_at.map(|d| d.to_rfc3339()),
        }
    }
}
