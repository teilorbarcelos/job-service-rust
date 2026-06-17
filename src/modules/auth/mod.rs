pub mod controller;
pub mod routes;
pub mod schemas;
pub mod service;

pub use routes::router;

use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        controller::login_handler,
        controller::get_me_handler,
        controller::logout_handler,
        controller::refresh_handler,
    ),
    components(schemas(
        schemas::LoginRequest,
        schemas::RefreshRequest,
        schemas::PermissionInfo,
        schemas::RoleInfo,
        schemas::UserInfo,
        schemas::AuthResponse,
        schemas::UserMeResponse,
        schemas::SimpleStatusResponse,
        schemas::RefreshResponse,
    ))
)]
pub struct AuthApi;
