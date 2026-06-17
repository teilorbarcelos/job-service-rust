use crate::common::{read_body_json, TestClient, TestContext};
use axum::http::StatusCode;
use serde_json::json;

pub async fn run(ctx: &TestContext) {
    println!("=== Running {{EntityName}} CRUD Tests ===");
    ctx.clear_database().await;
    ctx.reset_rate_limiter().await;

    let mut client = TestClient::new(ctx.router.clone());
    let login_payload = json!({
        "email": "admin@email.com",
        "password": "admin@123"
    });
    let (_, login_resp) = client.post_json("/v1/auth/login", &login_payload).await;
    let admin_token = read_body_json(login_resp).await["token"]
        .as_str()
        .unwrap()
        .to_string();

    client.set_token(Some(admin_token));

    // Test Create
    let create_payload = json!({{CreatePayloadJson}});
    let (status, resp) = client.post_json("/v1/{{entity_slug}}", &create_payload).await;
    assert_eq!(status, StatusCode::CREATED);
    let body = read_body_json(resp).await;
    let id = body["id"].as_str().unwrap().to_string();

    // Test Get
    let (status, resp) = client.get(&format!("/v1/{{entity_slug}}/{}", id)).await;
    assert_eq!(status, StatusCode::OK);
    let body = read_body_json(resp).await;
    {{AssertCreateFieldsJson}}

    // Test List
    let (status, _) = client.get("/v1/{{entity_slug}}/all").await;
    assert_eq!(status, StatusCode::OK);

    // Test List with search/filter/order
    let (status, _) = client.get("/v1/{{entity_slug}}?page=0&size=10").await;
    assert_eq!(status, StatusCode::OK);

    // Test Update
    let update_payload = json!({{UpdatePayloadJson}});
    let (status, resp) = client.put_json(&format!("/v1/{{entity_slug}}/{}", id), &update_payload).await;
    assert_eq!(status, StatusCode::OK);
    let body = read_body_json(resp).await;
    {{AssertUpdateFieldsJson}}

    // Test Toggle Status
    let toggle_payload = json!({
        "active": false
    });
    let (status, resp) = client.patch_json(&format!("/v1/{{entity_slug}}/{}/status", id), &toggle_payload).await;
    assert_eq!(status, StatusCode::OK);
    let body = read_body_json(resp).await;
    assert_eq!(body["active"].as_bool().unwrap(), false);

    // Test Get 404
    let (status, _) = client.get("/v1/{{entity_slug}}/non-existent-id").await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Test Update 404
    let (status, _) = client.put_json("/v1/{{entity_slug}}/non-existent-id", &update_payload).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Test Patch 404
    let (status, _) = client.patch_json("/v1/{{entity_slug}}/non-existent-id/status", &toggle_payload).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Test Delete 404
    let (status, _) = client.delete("/v1/{{entity_slug}}/non-existent-id").await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    // Test Delete
    let (status, _) = client.delete(&format!("/v1/{{entity_slug}}/{}", id)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    // Test Get after Delete (soft deleted, should return 404)
    let (status, _) = client.get(&format!("/v1/{{entity_slug}}/{}", id)).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}
