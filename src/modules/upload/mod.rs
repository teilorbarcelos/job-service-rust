pub mod controller;
pub mod routes;

pub use routes::router;

use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(controller::upload_file_handler,),
    components(schemas(controller::UploadResponse,))
)]
pub struct UploadApi;
