pub mod controller;
pub mod routes;
pub mod schemas;
pub mod service;

pub use routes::router;

use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(controller::get_stats_handler),
    components(schemas(
        schemas::DashboardStatsResponseDto,
        schemas::TimeSeriesStatDto,
        schemas::UserProductStatDto
    ))
)]
pub struct DashboardApi;
