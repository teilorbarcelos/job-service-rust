use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateUserRequest {
    pub name: String,
    pub email: String,
    pub password: String,
    pub id_role: String,
    pub phone: Option<String>,
    pub document: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateUserRequest {
    pub name: String,
    pub email: String,
    pub id_role: String,
    pub phone: Option<String>,
    pub document: Option<String>,
    pub active: Option<bool>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UserResponse {
    pub id: String,
    pub name: String,
    pub email: String,
    pub phone: Option<String>,
    pub document: Option<String>,
    pub active: bool,
    pub id_role: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PaginatedUserResponse {
    pub items: Vec<UserResponse>,
    pub total: u64,
    pub page: u64,
    pub size: u64,
}

impl From<crate::models::user::Model> for UserResponse {
    fn from(u: crate::models::user::Model) -> Self {
        Self {
            id: u.id,
            name: u.name,
            email: u.email,
            phone: u.phone,
            document: u.document,
            active: u.active,
            id_role: u.id_role,
            created_at: u.created_at.to_rfc3339(),
            updated_at: u.updated_at.to_rfc3339(),
        }
    }
}
