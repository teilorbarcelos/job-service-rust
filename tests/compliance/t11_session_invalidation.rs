use crate::common::{read_body_json, TestClient, TestContext};
use axum::http::StatusCode;
use serde_json::json;

pub async fn run(ctx: &TestContext) {
    println!("=== Running Session Invalidation Tests ===");
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
    admin_client.set_token(Some(admin_token));

    test_session_invalidated_on_role_deactivation(ctx, &mut admin_client).await;
    test_session_invalidated_on_role_update(ctx, &mut admin_client).await;
    test_session_invalidated_on_user_deactivation(ctx, &mut admin_client).await;
    test_session_invalidated_on_user_update(ctx, &mut admin_client).await;
}

async fn test_session_invalidated_on_role_deactivation(
    ctx: &TestContext,
    admin_client: &mut TestClient,
) {
    let unique_suffix = uuid::Uuid::new_v4().to_string()[..8].to_string();
    let email = format!("role_deact_{}@email.com", unique_suffix);
    let password = "Password123!";

    let role_payload = json!({
        "name": format!("Deact Role {}", unique_suffix),
        "description": "Will be deactivated",
        "permissions": []
    });
    let (_, r_resp) = admin_client.post_json("/v1/role", &role_payload).await;
    let role_id = read_body_json(r_resp).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    let user_payload = json!({
        "name": "Role Deact Test User",
        "email": email,
        "password": password,
        "id_role": role_id
    });
    let (_, u_resp) = admin_client.post_json("/v1/user", &user_payload).await;
    let user_id = read_body_json(u_resp).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    let mut user_client = TestClient::new(ctx.router.clone());
    let (_, login_resp) = user_client
        .post_json(
            "/v1/auth/login",
            &json!({ "email": email, "password": password }),
        )
        .await;
    let user_token = read_body_json(login_resp).await["token"]
        .as_str()
        .unwrap()
        .to_string();
    user_client.set_token(Some(user_token));

    let (me_status, _) = user_client.get("/v1/auth/me").await;
    assert_eq!(me_status, StatusCode::OK);

    let (status, _) = admin_client
        .patch_json(
            &format!("/v1/role/{}/status", role_id),
            &json!({ "active": false }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    let (me_status_after, _) = user_client.get("/v1/auth/me").await;
    assert_eq!(
        me_status_after,
        StatusCode::UNAUTHORIZED,
        "Session not invalidated on role deactivation"
    );

    let _ = admin_client.delete(&format!("/v1/user/{}", user_id)).await;
    let _ = admin_client.delete(&format!("/v1/role/{}", role_id)).await;
}

async fn test_session_invalidated_on_role_update(ctx: &TestContext, admin_client: &mut TestClient) {
    let unique_suffix = uuid::Uuid::new_v4().to_string()[..8].to_string();
    let email = format!("role_upd_{}@email.com", unique_suffix);
    let password = "Password123!";

    let role_payload = json!({
        "name": format!("Update Role {}", unique_suffix),
        "description": "Will be updated",
        "permissions": []
    });
    let (_, r_resp) = admin_client.post_json("/v1/role", &role_payload).await;
    let role_id = read_body_json(r_resp).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    let user_payload = json!({
        "name": "Role Update Test User",
        "email": email,
        "password": password,
        "id_role": role_id
    });
    let (_, u_resp) = admin_client.post_json("/v1/user", &user_payload).await;
    let user_id = read_body_json(u_resp).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    let mut user_client = TestClient::new(ctx.router.clone());
    let (_, login_resp) = user_client
        .post_json(
            "/v1/auth/login",
            &json!({ "email": email, "password": password }),
        )
        .await;
    let user_token = read_body_json(login_resp).await["token"]
        .as_str()
        .unwrap()
        .to_string();
    user_client.set_token(Some(user_token));

    let (me_status, _) = user_client.get("/v1/auth/me").await;
    assert_eq!(me_status, StatusCode::OK);

    let (status, _) = admin_client
        .put_json(
            &format!("/v1/role/{}", role_id),
            &json!({
                "name": format!("Update Role {}", unique_suffix),
                "description": "New description"
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    let (me_status_after, _) = user_client.get("/v1/auth/me").await;
    assert_eq!(
        me_status_after,
        StatusCode::UNAUTHORIZED,
        "Session not invalidated on role update"
    );

    let _ = admin_client.delete(&format!("/v1/user/{}", user_id)).await;
    let _ = admin_client.delete(&format!("/v1/role/{}", role_id)).await;
}

async fn test_session_invalidated_on_user_deactivation(
    ctx: &TestContext,
    admin_client: &mut TestClient,
) {
    let unique_suffix = uuid::Uuid::new_v4().to_string()[..8].to_string();
    let email = format!("user_deact_{}@email.com", unique_suffix);
    let password = "Password123!";

    let user_payload = json!({
        "name": "User Deact Test User",
        "email": email,
        "password": password,
        "id_role": "administrator"
    });
    let (_, u_resp) = admin_client.post_json("/v1/user", &user_payload).await;
    let user_id = read_body_json(u_resp).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    let mut user_client = TestClient::new(ctx.router.clone());
    let (_, login_resp) = user_client
        .post_json(
            "/v1/auth/login",
            &json!({ "email": email, "password": password }),
        )
        .await;
    let user_token = read_body_json(login_resp).await["token"]
        .as_str()
        .unwrap()
        .to_string();
    user_client.set_token(Some(user_token));

    let (me_status, _) = user_client.get("/v1/auth/me").await;
    assert_eq!(me_status, StatusCode::OK);

    let (status, _) = admin_client
        .patch_json(
            &format!("/v1/user/{}/status", user_id),
            &json!({ "active": false }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    let (me_status_after, _) = user_client.get("/v1/auth/me").await;
    assert_eq!(
        me_status_after,
        StatusCode::UNAUTHORIZED,
        "Session not invalidated on user deactivation"
    );

    let _ = admin_client.delete(&format!("/v1/user/{}", user_id)).await;
}

async fn test_session_invalidated_on_user_update(ctx: &TestContext, admin_client: &mut TestClient) {
    let unique_suffix = uuid::Uuid::new_v4().to_string()[..8].to_string();
    let email = format!("user_upd_{}@email.com", unique_suffix);
    let password = "Password123!";

    let user_payload = json!({
        "name": "User Update Test User",
        "email": email,
        "password": password,
        "id_role": "administrator"
    });
    let (_, u_resp) = admin_client.post_json("/v1/user", &user_payload).await;
    let user_id = read_body_json(u_resp).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    let mut user_client = TestClient::new(ctx.router.clone());
    let (_, login_resp) = user_client
        .post_json(
            "/v1/auth/login",
            &json!({ "email": email, "password": password }),
        )
        .await;
    let user_token = read_body_json(login_resp).await["token"]
        .as_str()
        .unwrap()
        .to_string();
    user_client.set_token(Some(user_token));

    let (me_status, _) = user_client.get("/v1/auth/me").await;
    assert_eq!(me_status, StatusCode::OK);

    let (status, _) = admin_client
        .put_json(
            &format!("/v1/user/{}", user_id),
            &json!({
                "name": "User Update Test User Updated Name",
                "email": email,
                "id_role": "administrator",
                "phone": "11999990000",
                "document": "00000000000"
            }),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    let (me_status_after, _) = user_client.get("/v1/auth/me").await;
    assert_eq!(
        me_status_after,
        StatusCode::UNAUTHORIZED,
        "Session not invalidated on user update"
    );

    let _ = admin_client.delete(&format!("/v1/user/{}", user_id)).await;
}
