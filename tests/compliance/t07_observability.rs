use crate::common::{read_body_json, read_body_string, TestClient, TestContext};
use axum::http::StatusCode;

pub async fn run(ctx: &TestContext) {
    println!("=== Running Observability Tests ===");
    ctx.reset_rate_limiter().await;

    test_health_check_endpoint(ctx).await;
    test_prometheus_metrics_endpoint(ctx).await;
    test_liveness_check_endpoint(ctx).await;
    test_health_check_redis_down(ctx).await;
}

async fn test_health_check_endpoint(ctx: &TestContext) {
    let mut client = TestClient::new(ctx.router.clone());
    let (status, resp) = client.get("/ready").await;
    assert_eq!(status, StatusCode::OK);

    let body = read_body_json(resp).await;
    assert_eq!(body["status"].as_str().unwrap(), "UP");
    assert_eq!(body["database"].as_str().unwrap(), "UP");
    assert_eq!(body["cache"].as_str().unwrap(), "UP");
}

async fn test_prometheus_metrics_endpoint(ctx: &TestContext) {
    let mut client = TestClient::new(ctx.router.clone());
    let (status, resp) = client.get("/metrics").await;
    assert_eq!(status, StatusCode::OK);

    let body_str = read_body_string(resp).await;
    assert!(
        body_str.contains("http_requests_total")
            || body_str.contains("axum_http_requests_total")
            || body_str.contains("process_cpu_seconds_total")
            || body_str.contains("http_request_duration_seconds"),
        "Metrics output did not contain expected HTTP metrics: {}",
        body_str
    );
}

async fn test_liveness_check_endpoint(ctx: &TestContext) {
    let mut client = TestClient::new(ctx.router.clone());
    let (status, resp) = client.get("/health").await;
    assert_eq!(status, StatusCode::OK);
    let body = read_body_json(resp).await;
    assert_eq!(body["status"].as_str().unwrap(), "UP");
}

async fn test_health_check_redis_down(ctx: &TestContext) {
    let dead_cache = backend_rust::infra::cache::Cache::new("redis://127.0.0.1:9999");
    let obs_router = backend_rust::modules::observability::router(ctx.db.clone(), dead_cache);
    let mut client = TestClient::new(obs_router);
    let (status, resp) = client.get("/ready").await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    let body = read_body_json(resp).await;
    assert_eq!(body["status"].as_str().unwrap(), "DOWN");
    assert_eq!(body["database"].as_str().unwrap(), "UP");
    assert_eq!(body["cache"].as_str().unwrap(), "DOWN");
}
