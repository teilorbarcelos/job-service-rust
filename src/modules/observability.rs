use crate::{
    infra::cache::Cache,
    modules::{
        audit::AuditApi, auth::AuthApi, dashboard::DashboardApi, product::ProductApi,
        role::RoleApi, upload::UploadApi, user::UserApi,
    },
};
use axum::{
    extract::Request, extract::State, http::StatusCode, middleware::Next, response::IntoResponse,
    response::Response, routing::get, Json, Router,
};
use prometheus::{Encoder, HistogramVec, IntCounterVec, TextEncoder};
use sea_orm::DatabaseConnection;
use serde_json::json;
use std::sync::OnceLock;
use std::time::Instant;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[derive(OpenApi)]
#[openapi(
    modifiers(&SecurityAddon),
    tags(
        (name = "Auth", description = "Authentication & Sessions"),
        (name = "User", description = "User Profiling & Soft Deletes"),
        (name = "Role", description = "RBAC Roles & Granular Scopes"),
        (name = "Product", description = "Product Catalog & Pricing"),
        (name = "Audit", description = "System Mutation Auditor Trail"),
        (name = "Debug", description = "Development & Diagnostics Tools"),
        (name = "Dashboard", description = "Dashboard Statistics"),
        (name = "Upload", description = "File Uploading & Storage")
    )
)]
struct ApiDoc;

struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi
            .components
            .get_or_insert_with(utoipa::openapi::Components::new);
        components.add_security_scheme(
            "bearerAuth",
            utoipa::openapi::security::SecurityScheme::Http(
                utoipa::openapi::security::HttpBuilder::new()
                    .scheme(utoipa::openapi::security::HttpAuthScheme::Bearer)
                    .bearer_format("JWT")
                    .build(),
            ),
        );
    }
}

fn http_requests_total() -> &'static IntCounterVec {
    static METRIC: OnceLock<IntCounterVec> = OnceLock::new();
    METRIC.get_or_init(|| {
        prometheus::register_int_counter_vec!(
            "http_requests_total",
            "Total number of HTTP requests processed.",
            &["method", "path", "status"]
        )
        .unwrap()
    })
}

fn http_request_duration_seconds() -> &'static HistogramVec {
    static METRIC: OnceLock<HistogramVec> = OnceLock::new();
    METRIC.get_or_init(|| {
        prometheus::register_histogram_vec!(
            "http_request_duration_seconds",
            "HTTP request latencies in seconds.",
            &["method", "path", "status"]
        )
        .unwrap()
    })
}

pub async fn track_metrics_middleware(req: Request, next: Next) -> Response {
    let method = req.method().to_string();
    let path = req.uri().path().to_string();

    if path == "/metrics"
        || path == "/health"
        || path == "/liveness"
        || path.starts_with("/v1/docs")
        || path.starts_with("/api-docs")
    {
        return next.run(req).await;
    }

    let start = Instant::now();
    let response = next.run(req).await;
    let duration = start.elapsed().as_secs_f64();
    let status = response.status().as_u16().to_string();

    http_requests_total()
        .with_label_values(&[&method, &path, &status])
        .inc();

    http_request_duration_seconds()
        .with_label_values(&[&method, &path, &status])
        .observe(duration);

    response
}

fn db_pool_connections() -> &'static prometheus::GaugeVec {
    static METRIC: OnceLock<prometheus::GaugeVec> = OnceLock::new();
    METRIC.get_or_init(|| {
        prometheus::register_gauge_vec!(
            "db_pool_connections",
            "Number of database connections in the pool",
            &["database", "state"]
        )
        .unwrap()
    })
}

async fn metrics_handler() -> impl IntoResponse {
    if let Some(db) = crate::infra::database::DB_CONN.get() {
        use sea_orm::{ConnectionTrait, Statement};
        if let Ok(results) = db
            .query_all(Statement::from_string(
                db.get_database_backend(),
                "SELECT state, count(*)::int4 FROM pg_stat_activity WHERE datname = current_database() GROUP BY state;".to_owned(),
            ))
            .await
        {
            let mut active = 0.0;
            let mut idle = 0.0;
            let mut total = 0.0;

            for row in results {
                let state: Option<String> = row.try_get("", "state").ok();
                let count: i32 = row.try_get("", "count").unwrap_or(0);
                let count_f = count as f64;
                total += count_f;

                match state.as_deref() {
                    Some("active") => active += count_f,
                    _ => idle += count_f,
                }
            }

            db_pool_connections().with_label_values(&["main", "total"]).set(total);
            db_pool_connections().with_label_values(&["main", "idle"]).set(idle);
            db_pool_connections().with_label_values(&["main", "active"]).set(active);
        }
    }

    let mut buffer = Vec::new();
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    encoder.encode(&metric_families, &mut buffer).unwrap();

    (
        StatusCode::OK,
        [("content-type", "text/plain; version=0.0.4; charset=utf-8")],
        buffer,
    )
}

async fn liveness_handler() -> Json<serde_json::Value> {
    Json(json!({ "status": "UP" }))
}

async fn ready_handler(
    State((db, cache)): State<(DatabaseConnection, Cache)>,
) -> impl IntoResponse {
    let db_ok = db.ping().await.is_ok();

    let cache_ok = if let Ok(mut conn) = cache.pool.get().await {
        redis::cmd("PING")
            .query_async::<_, String>(&mut conn)
            .await
            .is_ok()
    } else {
        false
    };

    let status = if db_ok && cache_ok { "UP" } else { "DOWN" };
    let code = if db_ok && cache_ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (
        code,
        Json(json!({
            "status": status,
            "database": if db_ok { "UP" } else { "DOWN" },
            "cache": if cache_ok { "UP" } else { "DOWN" }
        })),
    )
}

pub fn router(db: DatabaseConnection, cache: Cache) -> Router {
    let state = (db, cache);

    let mut openapi = ApiDoc::openapi();

    for api in [
        AuthApi::openapi(),
        UserApi::openapi(),
        RoleApi::openapi(),
        ProductApi::openapi(),
        AuditApi::openapi(),
        DashboardApi::openapi(),
        UploadApi::openapi(),
    ] {
        openapi.merge(api);
    }

    Router::new()
        .route("/health", get(liveness_handler))
        .route("/liveness", get(liveness_handler))
        .route("/ready", get(ready_handler))
        .route("/metrics", get(metrics_handler))
        .with_state(state)
        .merge(SwaggerUi::new("/v1/docs").url("/api-docs/openapi.json", openapi))
}
