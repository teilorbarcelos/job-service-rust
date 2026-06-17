use crate::common::{read_body_json, TestClient, TestContext};
use axum::http::StatusCode;
use backend_rust::models::error_log;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};
use serde_json::json;

pub async fn run(ctx: &TestContext) {
    println!("=== Running Error Logs Tests ===");
    ctx.clear_database().await;
    ctx.reset_rate_limiter().await;

    let mut client = TestClient::new(ctx.router.clone());
    let login_payload = json!({
        "email": "admin@email.com",
        "password": "admin@123"
    });
    let (_, login_resp) = client.post_json("/v1/auth/login", &login_payload).await;
    let admin_body = read_body_json(login_resp).await;
    let admin_token = admin_body["token"].as_str().unwrap().to_string();
    let admin_id = admin_body["user"]["id"].as_str().unwrap().to_string();

    client.set_token(Some(admin_token.clone()));

    test_unhandled_error_logged_to_db(ctx, &mut client, &admin_id, &admin_token).await;
    test_error_log_db_failure(ctx).await;
}

async fn test_unhandled_error_logged_to_db(
    ctx: &TestContext,
    client: &mut TestClient,
    admin_id: &str,
    admin_token: &str,
) {
    let invalid_payload = json!({
        "description": "Triggering centralized validation error logging"
    });
    let (status, _) = client.post_json("/v1/role", &invalid_payload).await;
    assert!(status == StatusCode::BAD_REQUEST || status == StatusCode::UNPROCESSABLE_ENTITY);

    let unique_suffix = uuid::Uuid::new_v4().to_string()[..8].to_string();
    let role_payload = json!({
        "name": format!("Limited User Role {}", unique_suffix),
        "description": "Test Role",
        "permissions": [
            {
                "id_feature": "product",
                "create": false,
                "view": true,
                "delete": false,
                "activate": false
            }
        ]
    });
    let (_, r_resp) = client.post_json("/v1/role", &role_payload).await;
    let role_id = read_body_json(r_resp).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    let email = format!("test_err_{}@email.com", unique_suffix);
    let user_payload = json!({
        "name": "Test User",
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
    let limited_body = read_body_json(login_resp_lim).await;
    let limited_token = limited_body["token"].as_str().unwrap().to_string();
    let limited_id = limited_body["user"]["id"].as_str().unwrap().to_string();
    limited_client.set_token(Some(limited_token));

    let (status_403, _) = limited_client
        .post_json(
            "/v1/product",
            &json!({
                "name": "Forbidden Product",
                "sku": "FORBIDDEN-SKU",
                "category": "Forbidden",
                "price": 10.0,
                "stock": 1,
                "description": "forbidden product"
            }),
        )
        .await;
    assert_eq!(status_403, StatusCode::FORBIDDEN);

    tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;

    let log_entry = error_log::Entity::find()
        .filter(error_log::Column::IdUser.eq(admin_id))
        .filter(error_log::Column::Source.like("%/v1/role%"))
        .order_by_desc(error_log::Column::CreatedAt)
        .one(&ctx.db)
        .await
        .unwrap();

    assert!(
        log_entry.is_some(),
        "No error log record found in audit.tb_error_log for user {} on /v1/role",
        admin_id
    );
    let entry = log_entry.unwrap();
    assert!(entry.source.unwrap().contains("/v1/role"));

    let rbac_entry = error_log::Entity::find()
        .filter(error_log::Column::IdUser.eq(&limited_id))
        .filter(error_log::Column::Source.like("%/v1/product%"))
        .order_by_desc(error_log::Column::CreatedAt)
        .one(&ctx.db)
        .await
        .unwrap();

    assert!(
        rbac_entry.is_some(),
        "No error log record found for the RBAC forbidden error on /v1/product"
    );
    let entry_rbac = rbac_entry.unwrap();
    assert_eq!(entry_rbac.id_user.unwrap(), limited_id);

    client.set_token(Some(admin_token.to_string()));
    let _ = client.delete(&format!("/v1/user/{}", user_id)).await;
    let _ = client.delete(&format!("/v1/role/{}", role_id)).await;
}

async fn test_error_log_db_failure(ctx: &TestContext) {
    let mut client = TestClient::new(ctx.router.clone());

    let admin_id = "user-admin-uuid-00000000000000000001";
    let (access_token, _) = backend_rust::infra::auth::AuthService::generate_tokens(
        admin_id,
        "admin@email.com",
        "administrator",
        &ctx.config.jwt_secret,
        3600,
    )
    .unwrap();

    ctx.cache
        .create_session(admin_id, &format!("access:{}", access_token), 3600)
        .await
        .unwrap();

    client.set_token(Some(access_token));

    let invalid_payload = json!({
        "description": "Triggering error log DB failure"
    });

    let (status, _) = client
        .post_json("/v1/role?FORCE_ERROR_LOG_DB_FAILURE=true", &invalid_payload)
        .await;
    assert!(status == StatusCode::BAD_REQUEST || status == StatusCode::UNPROCESSABLE_ENTITY);

    tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;
}
