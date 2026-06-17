use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RefreshRequest {
    #[serde(rename = "refreshToken")]
    pub refresh_token: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PermissionInfo {
    pub feature: String,
    pub create: bool,
    pub view: bool,
    pub activate: bool,
    pub delete: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RoleInfo {
    pub id: String,
    pub name: String,
    pub permissions: Vec<PermissionInfo>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UserInfo {
    pub id: String,
    pub name: String,
    pub email: String,
    pub role: RoleInfo,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AuthResponse {
    pub token: String,
    #[serde(rename = "refreshToken")]
    pub refresh_token: String,
    pub user: UserInfo,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UserMeResponse {
    pub user: UserInfo,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SimpleStatusResponse {
    pub status: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct RefreshResponse {
    pub token: String,
    #[serde(rename = "refreshToken")]
    pub refresh_token: String,
}
