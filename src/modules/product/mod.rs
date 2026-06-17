pub mod controller;
pub mod routes;
pub mod schemas;
pub mod service;

pub use routes::router;

use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        controller::list_products_handler,
        controller::get_product_handler,
        controller::create_product_handler,
        controller::update_product_handler,
        controller::delete_product_handler,
        controller::toggle_product_status_handler,
    ),
    components(schemas(
        schemas::CreateProductRequest,
        schemas::UpdateProductRequest,
        schemas::ProductResponse,
        schemas::PaginatedProductResponse,
        controller::ToggleStatusRequest
    ))
)]
pub struct ProductApi;
