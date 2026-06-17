use crate::common::{read_body_json, TestClient, TestContext};
use axum::http::StatusCode;
use serde_json::json;

pub async fn run(ctx: &TestContext) {
    println!("=== Running Status Toggling Tests ===");
    ctx.clear_database().await;
    ctx.reset_rate_limiter().await;

    let mut admin_client = TestClient::new(ctx.router.clone());
    let login_payload = json!({
        "email": "admin@email.com",
        "password": "admin@123"
    });
    let (_, login_resp) = admin_client
        .post_json("/v1/auth/login", &login_payload)
        .await;
    let admin_token = read_body_json(login_resp).await["token"]
        .as_str()
        .unwrap()
        .to_string();
    admin_client.set_token(Some(admin_token.clone()));

    let unique_suffix = uuid::Uuid::new_v4().to_string()[..8].to_string();

    let allowed_role_payload = json!({
        "name": format!("Allowed Role {}", unique_suffix),
        "description": "Role with activation allowed",
        "permissions": [
            { "id_feature": "product", "create": true, "view": true, "delete": true, "activate": true },
            { "id_feature": "role", "create": true, "view": true, "delete": true, "activate": true },
            { "id_feature": "user", "create": true, "view": true, "delete": true, "activate": true }
        ]
    });
    let (_, resp) = admin_client
        .post_json("/v1/role", &allowed_role_payload)
        .await;
    let allowed_role_id = read_body_json(resp).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    let allowed_user_email = format!("allowed_{}@email.com", unique_suffix);
    let allowed_user_payload = json!({
        "name": "Allowed Tester",
        "email": allowed_user_email,
        "password": "Password123!",
        "id_role": allowed_role_id
    });
    let (_, resp) = admin_client
        .post_json("/v1/user", &allowed_user_payload)
        .await;
    let allowed_user_id = read_body_json(resp).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    let forbidden_role_payload = json!({
        "name": format!("Forbidden Role {}", unique_suffix),
        "description": "Role with activation forbidden",
        "permissions": [
            { "id_feature": "product", "create": true, "view": true, "delete": true, "activate": false },
            { "id_feature": "role", "create": true, "view": true, "delete": true, "activate": false },
            { "id_feature": "user", "create": true, "view": true, "delete": true, "activate": false }
        ]
    });
    let (_, resp) = admin_client
        .post_json("/v1/role", &forbidden_role_payload)
        .await;
    let forbidden_role_id = read_body_json(resp).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    let forbidden_user_email = format!("forbidden_{}@email.com", unique_suffix);
    let forbidden_user_payload = json!({
        "name": "Forbidden Tester",
        "email": forbidden_user_email,
        "password": "Password123!",
        "id_role": forbidden_role_id
    });
    let (_, resp) = admin_client
        .post_json("/v1/user", &forbidden_user_payload)
        .await;
    let forbidden_user_id = read_body_json(resp).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    let mut allowed_client = TestClient::new(ctx.router.clone());
    let (_, login_resp_all) = allowed_client
        .post_json(
            "/v1/auth/login",
            &json!({ "email": allowed_user_email, "password": "Password123!" }),
        )
        .await;
    let allowed_token = read_body_json(login_resp_all).await["token"]
        .as_str()
        .unwrap()
        .to_string();
    allowed_client.set_token(Some(allowed_token));

    let mut forbidden_client = TestClient::new(ctx.router.clone());
    let (_, login_resp_forb) = forbidden_client
        .post_json(
            "/v1/auth/login",
            &json!({ "email": forbidden_user_email, "password": "Password123!" }),
        )
        .await;
    let forbidden_token = read_body_json(login_resp_forb).await["token"]
        .as_str()
        .unwrap()
        .to_string();
    forbidden_client.set_token(Some(forbidden_token));

    test_toggle_product_status_by_admin(ctx, &mut admin_client).await;
    test_toggle_role_status_by_admin(ctx, &mut admin_client).await;
    test_toggle_user_status_by_admin(ctx, &mut admin_client).await;

    test_toggle_product_status_rbac_forbidden(ctx, &mut forbidden_client).await;
    test_toggle_product_status_rbac_allowed(ctx, &mut allowed_client).await;

    test_toggle_role_status_rbac_forbidden(ctx, &mut forbidden_client).await;
    test_toggle_role_status_rbac_allowed(ctx, &mut allowed_client).await;

    test_toggle_user_status_rbac_forbidden(ctx, &mut forbidden_client).await;
    test_toggle_user_status_rbac_allowed(ctx, &mut allowed_client).await;

    let _ = admin_client
        .delete(&format!("/v1/user/{}", allowed_user_id))
        .await;
    let _ = admin_client
        .delete(&format!("/v1/user/{}", forbidden_user_id))
        .await;
    let _ = admin_client
        .delete(&format!("/v1/role/{}", allowed_role_id))
        .await;
    let _ = admin_client
        .delete(&format!("/v1/role/{}", forbidden_role_id))
        .await;
}

async fn test_toggle_product_status_by_admin(_ctx: &TestContext, client: &mut TestClient) {
    let unique_suffix = uuid::Uuid::new_v4().to_string()[..8].to_string();
    let payload = json!({
        "name": format!("Status Prod {}", unique_suffix),
        "sku": format!("SKU-STAT-{}", unique_suffix),
        "category": "Test",
        "price": 1.0,
        "stock": 1,
        "description": "Test"
    });
    let (_, resp) = client.post_json("/v1/product", &payload).await;
    let prod_id = read_body_json(resp).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    let (status, resp_deact) = client
        .patch_json(
            &format!("/v1/product/{}/status", prod_id),
            &json!({ "active": false }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(!read_body_json(resp_deact).await["active"]
        .as_bool()
        .unwrap());

    let (status, resp_act) = client
        .patch_json(
            &format!("/v1/product/{}/status", prod_id),
            &json!({ "active": true }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(read_body_json(resp_act).await["active"].as_bool().unwrap());

    let _ = client.delete(&format!("/v1/product/{}", prod_id)).await;
}

async fn test_toggle_role_status_by_admin(_ctx: &TestContext, client: &mut TestClient) {
    let unique_suffix = uuid::Uuid::new_v4().to_string()[..8].to_string();
    let payload = json!({
        "name": format!("Toggle Role {}", unique_suffix),
        "description": "Test description",
        "permissions": []
    });
    let (_, resp) = client.post_json("/v1/role", &payload).await;
    let role_id = read_body_json(resp).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    let (status, resp_deact) = client
        .patch_json(
            &format!("/v1/role/{}/status", role_id),
            &json!({ "active": false }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(!read_body_json(resp_deact).await["active"]
        .as_bool()
        .unwrap());

    let (status, resp_act) = client
        .patch_json(
            &format!("/v1/role/{}/status", role_id),
            &json!({ "active": true }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(read_body_json(resp_act).await["active"].as_bool().unwrap());

    let _ = client.delete(&format!("/v1/role/{}", role_id)).await;
}

async fn test_toggle_user_status_by_admin(_ctx: &TestContext, client: &mut TestClient) {
    let unique_suffix = uuid::Uuid::new_v4().to_string()[..8].to_string();
    let payload = json!({
        "name": "Toggle User Test",
        "email": format!("toggle_{}@email.com", unique_suffix),
        "password": "Password123!",
        "id_role": "administrator"
    });
    let (_, resp) = client.post_json("/v1/user", &payload).await;
    let user_id = read_body_json(resp).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    let (status, resp_deact) = client
        .patch_json(
            &format!("/v1/user/{}/status", user_id),
            &json!({ "active": false }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(!read_body_json(resp_deact).await["active"]
        .as_bool()
        .unwrap());

    let (status, resp_act) = client
        .patch_json(
            &format!("/v1/user/{}/status", user_id),
            &json!({ "active": true }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);
    assert!(read_body_json(resp_act).await["active"].as_bool().unwrap());

    let _ = client.delete(&format!("/v1/user/{}", user_id)).await;
}

async fn test_toggle_product_status_rbac_forbidden(_ctx: &TestContext, client: &mut TestClient) {
    let (status, _) = client
        .patch_json("/v1/product/some-uuid/status", &json!({ "active": false }))
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

async fn test_toggle_product_status_rbac_allowed(ctx: &TestContext, client: &mut TestClient) {
    let mut admin_client = TestClient::new(ctx.router.clone());
    let login_payload = json!({ "email": "admin@email.com", "password": "admin@123" });
    let (_, login_resp) = admin_client
        .post_json("/v1/auth/login", &login_payload)
        .await;
    let admin_token = read_body_json(login_resp).await["token"]
        .as_str()
        .unwrap()
        .to_string();
    admin_client.set_token(Some(admin_token));

    let unique_suffix = uuid::Uuid::new_v4().to_string()[..8].to_string();
    let payload = json!({
        "name": format!("Allowed Prod {}", unique_suffix),
        "sku": format!("SKU-ALL-{}", unique_suffix),
        "category": "Test",
        "price": 1.0,
        "stock": 1,
        "description": "Test"
    });
    let (_, resp) = admin_client.post_json("/v1/product", &payload).await;
    let prod_id = read_body_json(resp).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    let (status, _) = client
        .patch_json(
            &format!("/v1/product/{}/status", prod_id),
            &json!({ "active": false }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    let _ = admin_client
        .delete(&format!("/v1/product/{}", prod_id))
        .await;
}

async fn test_toggle_role_status_rbac_forbidden(_ctx: &TestContext, client: &mut TestClient) {
    let (status, _) = client
        .patch_json("/v1/role/some-role/status", &json!({ "active": false }))
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

async fn test_toggle_role_status_rbac_allowed(ctx: &TestContext, client: &mut TestClient) {
    let mut admin_client = TestClient::new(ctx.router.clone());
    let login_payload = json!({ "email": "admin@email.com", "password": "admin@123" });
    let (_, login_resp) = admin_client
        .post_json("/v1/auth/login", &login_payload)
        .await;
    let admin_token = read_body_json(login_resp).await["token"]
        .as_str()
        .unwrap()
        .to_string();
    admin_client.set_token(Some(admin_token));

    let unique_suffix = uuid::Uuid::new_v4().to_string()[..8].to_string();
    let payload = json!({
        "name": format!("Allowed Role {}", unique_suffix),
        "description": "Test role description",
        "permissions": []
    });
    let (_, resp) = admin_client.post_json("/v1/role", &payload).await;
    let role_id = read_body_json(resp).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    let (status, _) = client
        .patch_json(
            &format!("/v1/role/{}/status", role_id),
            &json!({ "active": false }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    let _ = admin_client.delete(&format!("/v1/role/{}", role_id)).await;
}

async fn test_toggle_user_status_rbac_forbidden(_ctx: &TestContext, client: &mut TestClient) {
    let (status, _) = client
        .patch_json("/v1/user/some-user/status", &json!({ "active": false }))
        .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

async fn test_toggle_user_status_rbac_allowed(ctx: &TestContext, client: &mut TestClient) {
    let mut admin_client = TestClient::new(ctx.router.clone());
    let login_payload = json!({ "email": "admin@email.com", "password": "admin@123" });
    let (_, login_resp) = admin_client
        .post_json("/v1/auth/login", &login_payload)
        .await;
    let admin_token = read_body_json(login_resp).await["token"]
        .as_str()
        .unwrap()
        .to_string();
    admin_client.set_token(Some(admin_token));

    let unique_suffix = uuid::Uuid::new_v4().to_string()[..8].to_string();
    let payload = json!({
        "name": "Allowed Toggle User",
        "email": format!("allowed_toggle_{}@email.com", unique_suffix),
        "password": "Password123!",
        "id_role": "administrator"
    });
    let (_, resp) = admin_client.post_json("/v1/user", &payload).await;
    let user_id = read_body_json(resp).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    let (status, _) = client
        .patch_json(
            &format!("/v1/user/{}/status", user_id),
            &json!({ "active": false }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    let _ = admin_client.delete(&format!("/v1/user/{}", user_id)).await;
}
