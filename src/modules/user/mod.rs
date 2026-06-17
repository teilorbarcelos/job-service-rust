pub mod controller;
pub mod routes;
pub mod schemas;
pub mod service;

pub use routes::router;

use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        controller::list_users_handler,
        controller::get_user_handler,
        controller::create_user_handler,
        controller::update_user_handler,
        controller::delete_user_handler,
        controller::toggle_user_status_handler,
        controller::export_pdf_handler,
    ),
    components(schemas(
        schemas::CreateUserRequest,
        schemas::UpdateUserRequest,
        schemas::UserResponse,
        schemas::PaginatedUserResponse
    ))
)]
pub struct UserApi;
