pub mod controller;
pub mod routes;
pub mod schemas;
pub mod service;

pub use routes::router;

use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        controller::list_{{entity_slug}}s_handler,
        controller::get_{{entity_slug}}_handler,
        controller::create_{{entity_slug}}_handler,
        controller::update_{{entity_slug}}_handler,
        controller::delete_{{entity_slug}}_handler,
        controller::toggle_{{entity_slug}}_status_handler,
    ),
    components(schemas(
        schemas::Create{{EntityName}}Request,
        schemas::Update{{EntityName}}Request,
        schemas::{{EntityName}}Response,
        schemas::Paginated{{EntityName}}Response,
        controller::ToggleStatusRequest
    ))
)]
pub struct {{EntityName}}Api;
