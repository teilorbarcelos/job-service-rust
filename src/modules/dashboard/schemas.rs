use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DashboardStatsResponseDto {
    pub user_creation_stats: Vec<TimeSeriesStatDto>,
    pub product_creation_stats: Vec<TimeSeriesStatDto>,
    pub products_per_user: Vec<UserProductStatDto>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TimeSeriesStatDto {
    pub date: String,
    pub count: i32,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserProductStatDto {
    pub user_id: Option<String>,
    pub user_name: String,
    pub count: i32,
}
