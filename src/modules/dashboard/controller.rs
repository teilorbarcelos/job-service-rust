use crate::{
    errors::AppError, infra::cache::Cache, modules::dashboard::schemas::DashboardStatsResponseDto,
    modules::dashboard::service::DashboardModuleService,
};
use axum::{
    extract::{Query, State},
    Json,
};
use chrono::{DateTime, Duration, FixedOffset, NaiveDate, TimeZone, Utc};
use sea_orm::DatabaseConnection;

#[derive(Debug, serde::Deserialize)]
pub struct DashboardQuery {
    #[serde(rename = "createdAt_start")]
    pub created_at_start: Option<String>,
    #[serde(rename = "createdAt_end")]
    pub created_at_end: Option<String>,
}

fn parse_start_date(val: Option<&str>, default_days_ago: i64) -> DateTime<Utc> {
    let offset = FixedOffset::west_opt(3 * 3600).unwrap();
    if let Some(s) = val {
        if !s.is_empty() {
            if let Ok(naive_date) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
                let local_dt = offset
                    .from_local_datetime(&naive_date.and_hms_opt(0, 0, 0).unwrap())
                    .unwrap();
                return local_dt.with_timezone(&Utc);
            }
        }
    }
    let local_now = Utc::now().with_timezone(&offset);
    let local_start = (local_now - Duration::days(default_days_ago))
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    offset
        .from_local_datetime(&local_start)
        .unwrap()
        .with_timezone(&Utc)
}

fn parse_end_date(val: Option<&str>) -> DateTime<Utc> {
    let offset = FixedOffset::west_opt(3 * 3600).unwrap(); // UTC-3 (America/Sao_Paulo)
    if let Some(s) = val {
        if !s.is_empty() {
            if let Ok(naive_date) = NaiveDate::parse_from_str(s, "%Y-%m-%d") {
                let local_dt = offset
                    .from_local_datetime(&naive_date.and_hms_opt(23, 59, 59).unwrap())
                    .unwrap();
                return local_dt.with_timezone(&Utc);
            }
        }
    }
    Utc::now()
}

#[utoipa::path(
    get,
    path = "/v1/dashboard/stats",
    params(
        ("createdAt_start" = Option<String>, Query, description = "Filter start date (YYYY-MM-DD)"),
        ("createdAt_end" = Option<String>, Query, description = "Filter end date (YYYY-MM-DD)"),
    ),
    responses(
        (status = 200, description = "Dashboard stats retrieved successfully", body = DashboardStatsResponseDto),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearerAuth" = [])
    ),
    tag = "Dashboard"
)]
pub async fn get_stats_handler(
    State(state): State<(DatabaseConnection, Cache, crate::config::AppConfig)>,
    Query(query): Query<DashboardQuery>,
) -> Result<Json<DashboardStatsResponseDto>, AppError> {
    let (db, _, _) = state;

    let start = parse_start_date(query.created_at_start.as_deref(), 30);
    let end = parse_end_date(query.created_at_end.as_deref());

    if start > end {
        return Err(AppError::BadRequest(
            "A data de início deve ser anterior ou igual à data de fim".to_string(),
        ));
    }

    let stats = DashboardModuleService::get_stats(start, end, &db).await?;
    Ok(Json(stats))
}
