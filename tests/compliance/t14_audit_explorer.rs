use crate::common::{read_body_json, read_body_string, TestClient, TestContext};
use axum::http::StatusCode;
use serde_json::json;

pub async fn run(ctx: &TestContext) {
    println!("=== Running Audit Explorer Tests ===");
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

    let (html_status, html_resp) = client.get("/admin/logs").await;
    assert_eq!(html_status, StatusCode::OK);
    let html_content = read_body_string(html_resp).await;
    assert!(html_content.contains("<!DOCTYPE html>"));
    assert!(html_content.contains("Explorer"));

    client.set_token(Some(admin_token.clone()));

    let unique_suffix = uuid::Uuid::new_v4().to_string()[..8].to_string();
    let payload = json!({
        "name": format!("Audit Explorer Role {}", unique_suffix),
        "description": "Trigger audit",
        "permissions": []
    });
    let (c_status, c_resp) = client.post_json("/v1/role", &payload).await;
    assert_eq!(c_status, StatusCode::CREATED);
    let audit_role_id = read_body_json(c_resp).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    let bad_payload = json!({
        "description": "Trigger error log"
    });
    let (b_status, _) = client.post_json("/v1/role", &bad_payload).await;
    assert_eq!(b_status, StatusCode::BAD_REQUEST);

    tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

    let (status, resp) = client.get("/admin/api/audit?page=0&size=5").await;
    assert_eq!(status, StatusCode::OK);
    let body = read_body_json(resp).await;
    assert!(body["items"].is_array());
    assert!(body["total"].is_number());

    let (status_search, resp_search) = client.get("/admin/api/audit?search=admin").await;
    assert_eq!(status_search, StatusCode::OK);
    let body_search = read_body_json(resp_search).await;
    assert!(body_search["items"].is_array());

    let (status_err, resp_err) = client.get("/admin/api/errors?page=0&size=5").await;
    assert_eq!(status_err, StatusCode::OK);
    let body_err = read_body_json(resp_err).await;
    assert!(body_err["items"].is_array());

    let (status_err_search, resp_err_search) = client.get("/admin/api/errors?search=test").await;
    assert_eq!(status_err_search, StatusCode::OK);
    let body_err_search = read_body_json(resp_err_search).await;
    assert!(body_err_search["items"].is_array());

    let unique_suffix = uuid::Uuid::new_v4().to_string()[..8].to_string();
    let role_payload = json!({
        "name": format!("Limited Role {}", unique_suffix),
        "description": "limited role",
        "permissions": []
    });
    let (_, r_resp) = client.post_json("/v1/role", &role_payload).await;
    let role_id = read_body_json(r_resp).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    let email = format!("explorer_lim_{}@email.com", unique_suffix);
    let user_payload = json!({
        "name": "Explorer Limited User",
        "email": email,
        "password": "Password123!",
        "id_role": role_id
    });
    let (_, u_resp) = client.post_json("/v1/user", &user_payload).await;
    let user_id = read_body_json(u_resp).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    let mut limited_client = TestClient::new(ctx.router.clone());
    let (_, login_resp_lim) = limited_client
        .post_json(
            "/v1/auth/login",
            &json!({ "email": email, "password": "Password123!" }),
        )
        .await;
    let limited_token = read_body_json(login_resp_lim).await["token"]
        .as_str()
        .unwrap()
        .to_string();
    limited_client.set_token(Some(limited_token));

    let (forbidden_status_audit, _) = limited_client.get("/admin/api/audit").await;
    assert_eq!(forbidden_status_audit, StatusCode::FORBIDDEN);

    let (forbidden_status_errors, _) = limited_client.get("/admin/api/errors").await;
    assert_eq!(forbidden_status_errors, StatusCode::FORBIDDEN);

    let _ = client.delete(&format!("/v1/user/{}", user_id)).await;
    let _ = client.delete(&format!("/v1/role/{}", role_id)).await;
    let _ = client.delete(&format!("/v1/role/{}", audit_role_id)).await;
}
