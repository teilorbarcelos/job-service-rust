use crate::common::{read_body_json, read_body_string, TestClient, TestContext};
use axum::http::StatusCode;
use serde_json::json;

pub async fn run(ctx: &TestContext) {
    println!("=== Running PDF Export & Debug Removal Tests ===");
    ctx.clear_database().await;
    ctx.reset_rate_limiter().await;

    test_pdf_debug_endpoints_removed(ctx).await;
    test_pdf_export_security_and_functionality(ctx).await;
}

async fn test_pdf_debug_endpoints_removed(ctx: &TestContext) {
    let mut client = TestClient::new(ctx.router.clone());

    // 1. In dev environment config (default test config is development)
    let (status_get, _) = client.get("/v1/debug/pdf").await;
    assert_eq!(status_get, StatusCode::NOT_FOUND);

    let (status_post, _) = client
        .request("POST", "/v1/debug/pdf", axum::body::Body::empty(), None)
        .await;
    assert_eq!(status_post, StatusCode::NOT_FOUND);

    // 2. In production environment config
    let mut prod_config = ctx.config.clone();
    prod_config.environment = "production".to_string();
    let prod_router =
        backend_rust::modules::app_router(ctx.db.clone(), ctx.cache.clone(), prod_config);
    let mut prod_client = TestClient::new(prod_router);

    let (status_prod_get, _) = prod_client.get("/v1/debug/pdf").await;
    assert_eq!(status_prod_get, StatusCode::NOT_FOUND);

    let (status_prod_post, _) = prod_client
        .request("POST", "/v1/debug/pdf", axum::body::Body::empty(), None)
        .await;
    assert_eq!(status_prod_post, StatusCode::NOT_FOUND);
}

async fn test_pdf_export_security_and_functionality(ctx: &TestContext) {
    let mut client = TestClient::new(ctx.router.clone());

    // 1. Unauthenticated request to export PDF
    let (status_unauth, _) = client.get("/v1/user/export/pdf").await;
    assert_eq!(status_unauth, StatusCode::UNAUTHORIZED);

    // Get admin token
    let login_payload = json!({
        "email": "admin@email.com",
        "password": "admin@123"
    });
    let (_, login_resp) = client.post_json("/v1/auth/login", &login_payload).await;
    let admin_token = read_body_json(login_resp).await["token"]
        .as_str()
        .unwrap()
        .to_string();

    // 2. Unauthorized request (create user/role with no view permission for user feature)
    client.set_token(Some(admin_token.clone()));
    let unique_suffix = uuid::Uuid::new_v4().to_string()[..8].to_string();

    let role_payload = json!({
        "name": format!("Limited Tester {}", unique_suffix),
        "description": "Test Role",
        "permissions": [
            {
                "id_feature": "user",
                "create": false,
                "view": false,
                "delete": false,
                "activate": false
            }
        ]
    });
    let (r_status, r_resp) = client.post_json("/v1/role", &role_payload).await;
    assert_eq!(r_status, StatusCode::CREATED);
    let role_id = read_body_json(r_resp).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    let email = format!("test_pdf_{}@email.com", unique_suffix);
    let user_payload = json!({
        "name": "Test User PDF",
        "email": email,
        "password": "Password123!",
        "id_role": role_id
    });
    let (u_status, _) = client.post_json("/v1/user", &user_payload).await;
    assert_eq!(u_status, StatusCode::CREATED);

    // Login as limited user
    client.set_token(None);
    let limited_login_payload = json!({
        "email": email,
        "password": "Password123!"
    });
    let (login_status, login_resp_limited) = client
        .post_json("/v1/auth/login", &limited_login_payload)
        .await;
    assert_eq!(login_status, StatusCode::OK);
    let limited_token = read_body_json(login_resp_limited).await["token"]
        .as_str()
        .unwrap()
        .to_string();

    // Test forbidden
    client.set_token(Some(limited_token));
    let (status_forbidden, _) = client.get("/v1/user/export/pdf").await;
    assert_eq!(status_forbidden, StatusCode::FORBIDDEN);

    // 3. Authorized request (admin)
    client.set_token(Some(admin_token));

    // Insert 105 users to trigger pagination page increment
    use backend_rust::models::user;
    use sea_orm::{EntityTrait, Set};
    let mut users_to_insert = Vec::new();
    for i in 0..105 {
        let unique_id = uuid::Uuid::new_v4().to_string();
        let active_user = user::ActiveModel {
            id: Set(unique_id),
            name: Set(format!("Bulk User {}", i)),
            email: Set(format!("bulk_user_{}_{}@email.com", i, unique_suffix)),
            phone: Set(None),
            cognito_id: Set(None),
            active: Set(true),
            created_at: Set(chrono::Utc::now().into()),
            updated_at: Set(chrono::Utc::now().into()),
            document: Set(None),
            is_deleted: Set(Some(false)),
            deleted_at: Set(None),
            avatar: Set(None),
            id_auth: Set(None),
            id_role: Set(role_id.clone()),
        };
        users_to_insert.push(active_user);
    }
    user::Entity::insert_many(users_to_insert)
        .exec(&ctx.db)
        .await
        .unwrap();

    // First, request without query params
    let (status_ok, resp_ok) = client.get("/v1/user/export/pdf").await;
    if status_ok != StatusCode::OK {
        let body_err = read_body_string(resp_ok).await;
        panic!(
            "GET /v1/user/export/pdf failed with status: {}, error: {}",
            status_ok, body_err
        );
    }

    let headers = resp_ok.headers();
    assert_eq!(
        headers.get("Content-Type").unwrap().to_str().unwrap(),
        "application/pdf"
    );
    let disposition = headers
        .get("Content-Disposition")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(disposition.contains("attachment"));
    assert!(disposition.contains("filename=\"usuarios.pdf\""));

    let body_bytes = axum::body::to_bytes(resp_ok.into_body(), usize::MAX)
        .await
        .unwrap();
    assert!(body_bytes.starts_with(b"%PDF"));

    // Next, request with query params to cover filtering paths
    let (status_ok_query, resp_ok_query) = client
        .get("/v1/user/export/pdf?searchWord=admin&searchFields=name&orderBy=name&orderDirection=asc&active=true")
        .await;
    if status_ok_query != StatusCode::OK {
        let body_err = read_body_string(resp_ok_query).await;
        panic!(
            "GET /v1/user/export/pdf with query failed with status: {}, error: {}",
            status_ok_query, body_err
        );
    }

    let body_bytes_query = axum::body::to_bytes(resp_ok_query.into_body(), usize::MAX)
        .await
        .unwrap();
    assert!(body_bytes_query.starts_with(b"%PDF"));
}
