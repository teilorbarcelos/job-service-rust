use crate::common::{TestClient, TestContext};
use axum::http::StatusCode;

pub async fn run(ctx: &TestContext) {
    println!("=== Running Rate Limit Tests ===");
    ctx.reset_rate_limiter().await;

    test_rate_limit_headers(ctx).await;
    test_rate_limit_exceeded(ctx).await;
}

async fn test_rate_limit_headers(ctx: &TestContext) {
    let mut client = TestClient::new(ctx.router.clone());
    let (status, resp) = client.get("/health").await;
    assert_eq!(status, StatusCode::OK);

    let headers = resp.headers();
    let limit = headers
        .get("x-ratelimit-limit")
        .expect("Missing X-RateLimit-Limit header")
        .to_str()
        .expect("Invalid limit header value");
    let remaining = headers
        .get("x-ratelimit-remaining")
        .expect("Missing X-RateLimit-Remaining header")
        .to_str()
        .expect("Invalid remaining header value");

    let limit_val: i64 = limit.parse().expect("Limit is not an integer");
    let remaining_val: i64 = remaining.parse().expect("Remaining is not an integer");

    assert!(limit_val > 0, "RateLimit limit should be positive");
    assert!(
        remaining_val >= 0,
        "RateLimit remaining should be non-negative"
    );
}

async fn test_rate_limit_exceeded(ctx: &TestContext) {
    let redis_client = redis::Client::open(ctx.config.redis_url.as_str()).unwrap();
    let mut conn = redis_client.get_async_connection().await.unwrap();
    let ip = "192.168.1.100";
    let redis_key = format!("ratelimit:{}", ip);

    let _: () = redis::cmd("DEL")
        .arg(&redis_key)
        .query_async(&mut conn)
        .await
        .unwrap();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64;
    let mut pipe = redis::pipe();
    for i in 0..1005 {
        pipe.cmd("ZADD")
            .arg(&redis_key)
            .arg(now + i)
            .arg(format!("val-{}", i));
    }
    let _: () = pipe.query_async(&mut conn).await.unwrap();

    let mut client = TestClient::new(ctx.router.clone());
    let (status, resp) = client
        .request_with_headers(
            "GET",
            "/health",
            axum::body::Body::empty(),
            None,
            vec![("X-Forwarded-For", ip)],
        )
        .await;

    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);

    let headers = resp.headers();
    assert!(headers.get("x-ratelimit-limit").is_some());
    assert_eq!(
        headers
            .get("x-ratelimit-remaining")
            .unwrap()
            .to_str()
            .unwrap(),
        "0"
    );

    let (status_tracked, _) = client
        .request_with_headers(
            "GET",
            "/v1/nonexistent-route-for-metrics-test",
            axum::body::Body::empty(),
            None,
            vec![("X-Forwarded-For", ip)],
        )
        .await;
    assert_eq!(status_tracked, StatusCode::TOO_MANY_REQUESTS);

    let (metrics_status, metrics_resp) = client.get("/metrics").await;
    assert_eq!(metrics_status, StatusCode::OK);
    let metrics_str = crate::common::read_body_string(metrics_resp).await;
    assert!(
        metrics_str.contains("status=\"429\""),
        "Expected metrics to record a 429 error, but found:\n{}",
        metrics_str
    );

    let _: () = redis::cmd("DEL")
        .arg(&redis_key)
        .query_async(&mut conn)
        .await
        .unwrap();
}
