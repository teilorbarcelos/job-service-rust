use crate::{
    config::AppConfig,
    errors::{AppError, AppJson},
    infra::cache::Cache,
    middleware::auth::CurrentUser,
    modules::auth::schemas::{
        AuthResponse, LoginRequest, RefreshRequest, SimpleStatusResponse, UserMeResponse,
    },
    modules::auth::service::AuthModuleService,
};
use axum::{extract::State, Extension, Json};
use sea_orm::DatabaseConnection;

#[utoipa::path(
    post,
    path = "/v1/auth/login",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "User authenticated successfully", body = AuthResponse),
        (status = 400, description = "Invalid credentials or request data")
    ),
    tag = "Auth"
)]
pub async fn login_handler(
    State(state): State<(DatabaseConnection, Cache, AppConfig)>,
    AppJson(payload): AppJson<LoginRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let (db, cache, config) = state;
    let auth_data = AuthModuleService::login(payload, &db, &cache, &config).await?;
    Ok(Json(auth_data))
}

#[utoipa::path(
    get,
    path = "/v1/auth/me",
    responses(
        (status = 200, description = "Current user retrieved successfully", body = UserMeResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearerAuth" = [])
    ),
    tag = "Auth"
)]
pub async fn get_me_handler(
    State(state): State<(DatabaseConnection, Cache, AppConfig)>,
    Extension(current_user): Extension<CurrentUser>,
) -> Result<Json<UserMeResponse>, AppError> {
    let (db, _, _) = state;
    let me_data = AuthModuleService::get_me(&current_user.id, &db).await?;
    Ok(Json(me_data))
}

#[utoipa::path(
    post,
    path = "/v1/auth/logout",
    responses(
        (status = 200, description = "Logged out successfully", body = SimpleStatusResponse),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearerAuth" = [])
    ),
    tag = "Auth"
)]
pub async fn logout_handler(
    State(state): State<(DatabaseConnection, Cache, AppConfig)>,
    Extension(current_user): Extension<CurrentUser>,
) -> Result<Json<SimpleStatusResponse>, AppError> {
    let (_, cache, _) = state;
    let response = AuthModuleService::logout(&current_user.id, &cache).await?;
    Ok(Json(response))
}

#[utoipa::path(
    post,
    path = "/v1/auth/refresh",
    request_body = RefreshRequest,
    responses(
        (status = 200, description = "Token refreshed successfully", body = AuthResponse),
        (status = 400, description = "Invalid refresh token")
    ),
    tag = "Auth"
)]
pub async fn refresh_handler(
    State(state): State<(DatabaseConnection, Cache, AppConfig)>,
    AppJson(payload): AppJson<RefreshRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let (db, cache, config) = state;
    let auth_data =
        AuthModuleService::refresh(&payload.refresh_token, &db, &cache, &config).await?;
    Ok(Json(auth_data))
}
