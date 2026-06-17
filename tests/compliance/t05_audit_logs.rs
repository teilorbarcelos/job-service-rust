use crate::common::{read_body_json, TestClient, TestContext};
use axum::http::StatusCode;
use backend_rust::models::audit;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde_json::json;

pub async fn run(ctx: &TestContext) {
    println!("=== Running Audit Logs Tests ===");
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

    test_audit_log_created_on_mutation(ctx, &mut client).await;
    test_audit_log_ignores_unauthenticated_requests(ctx, &mut client).await;
    test_audit_log_password_scrubbing(ctx, &mut client).await;
    test_list_audit_logs(ctx, &mut client).await;
    test_audit_log_db_failure(ctx).await;
}

async fn test_audit_log_created_on_mutation(ctx: &TestContext, client: &mut TestClient) {
    let payload = json!({
        "name": "Administrador",
        "description": "Updated by audit test"
    });
    let (status, _) = client.put_json("/v1/role/administrator", &payload).await;
    assert_eq!(status, StatusCode::OK, "Failed to update role");

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let log_entry = audit::Entity::find()
        .filter(audit::Column::TableName.eq("Role"))
        .order_by_desc(audit::Column::CreatedAt)
        .one(&ctx.db)
        .await
        .unwrap();

    assert!(
        log_entry.is_some(),
        "No audit log found in DB after mutation"
    );
    let entry = log_entry.unwrap();
    assert_eq!(entry.method, "PUT");
    assert_eq!(entry.action_type, "UPDATE");
    assert!(!entry.diff_value.is_empty());
}

async fn test_audit_log_ignores_unauthenticated_requests(
    ctx: &TestContext,
    client: &mut TestClient,
) {
    let count_before = audit::Entity::find().all(&ctx.db).await.unwrap().len();

    client.set_token(None);
    let payload = json!({
        "email": "invalid@example.com",
        "password": "wrong"
    });
    let (status, _) = client.post_json("/v1/auth/login", &payload).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let count_after = audit::Entity::find().all(&ctx.db).await.unwrap().len();
    assert_eq!(
        count_before, count_after,
        "Unauthenticated requests should not trigger audit logs"
    );
}

async fn test_audit_log_password_scrubbing(ctx: &TestContext, client: &mut TestClient) {
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

    let unique_suffix = uuid::Uuid::new_v4().to_string()[..8].to_string();
    let test_password = format!("SuperSecret{}!", unique_suffix);
    let test_email = format!("audit_test_{}@email.com", unique_suffix);

    let payload = json!({
        "name": "Audit Test User",
        "email": test_email,
        "password": test_password,
        "id_role": "administrator"
    });

    let (status, resp) = client.post_json("/v1/user", &payload).await;
    assert_eq!(status, StatusCode::CREATED);
    let user_id = read_body_json(resp).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let log_entry = audit::Entity::find()
        .filter(audit::Column::TableName.eq("User"))
        .filter(audit::Column::Method.eq("POST"))
        .order_by_desc(audit::Column::CreatedAt)
        .one(&ctx.db)
        .await
        .unwrap();

    assert!(
        log_entry.is_some(),
        "Audit log for user creation was not found"
    );
    let entry = log_entry.unwrap();

    assert!(
        !entry.raw.contains(&test_password),
        "Password leaked into the raw audit log!"
    );
    assert!(
        !entry.params.contains(&test_password),
        "Password leaked into the params audit log!"
    );

    let (d_status, _) = client.delete(&format!("/v1/user/{}", user_id)).await;
    assert_eq!(d_status, StatusCode::NO_CONTENT);
}

async fn test_list_audit_logs(_ctx: &TestContext, client: &mut TestClient) {
    let (status, resp) = client.get("/v1/audit/all").await;
    assert_eq!(status, StatusCode::OK);
    let body = read_body_json(resp).await;
    assert!(body["total"].as_u64().is_some());
    assert!(body["items"].as_array().is_some());

    let (status, resp) = client.get("/v1/audit/all?page=0&size=10&searchWord=admin&searchFields=username&orderBy=createdAt&orderDirection=desc").await;
    assert_eq!(status, StatusCode::OK);
    let body = read_body_json(resp).await;
    assert!(body["items"].as_array().is_some());

    let (status, _) = client
        .get("/v1/audit/all?searchFields=invalid_field&searchWord=test")
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (status, _) = client.get("/v1/audit/all?orderBy=invalid_order").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

async fn test_audit_log_db_failure(ctx: &TestContext) {
    let mut client = TestClient::new(ctx.router.clone());

    let long_email = "a".repeat(300);
    let (access_token, _) = backend_rust::infra::auth::AuthService::generate_tokens(
        "user-admin-uuid-00000000000000000001",
        &long_email,
        "administrator",
        &ctx.config.jwt_secret,
        3600,
    )
    .unwrap();

    ctx.cache
        .create_session(
            "user-admin-uuid-00000000000000000001",
            &format!("access:{}", access_token),
            3600,
        )
        .await
        .unwrap();

    client.set_token(Some(access_token));

    let unique_suffix = uuid::Uuid::new_v4().to_string()[..8].to_string();
    let payload = json!({
        "name": format!("Audit Failure Role {}", unique_suffix),
        "description": "Trigger audit DB error",
        "permissions": []
    });

    let (status, resp) = client.post_json("/v1/role", &payload).await;
    let body = read_body_json(resp).await;
    println!(
        "test_audit_log_db_failure response status: {:?}, body: {:?}",
        status, body
    );
    assert_eq!(status, StatusCode::CREATED);

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
}
