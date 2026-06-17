pub mod controller;
pub mod routes;
pub mod schemas;
pub mod service;

pub use routes::router;

use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        controller::list_roles_handler,
        controller::get_role_handler,
        controller::create_role_handler,
        controller::update_role_handler,
        controller::delete_role_handler,
        controller::toggle_role_status_handler,
        controller::list_features_handler,
    ),
    components(schemas(
        schemas::PermissionRequest,
        schemas::CreateRoleRequest,
        schemas::UpdateRoleRequest,
        schemas::RoleResponse,
        schemas::PaginatedRoleResponse,
        schemas::FeatureResponse,
        controller::ToggleStatusRequest
    ))
)]
pub struct RoleApi;
