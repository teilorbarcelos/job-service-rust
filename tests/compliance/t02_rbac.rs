use crate::common::{read_body_json, TestClient, TestContext};
use axum::http::StatusCode;
use serde_json::json;

pub async fn run(ctx: &TestContext) {
    println!("=== Running RBAC Tests ===");
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

    let unique_suffix = uuid::Uuid::new_v4().to_string()[..8].to_string();

    let role_payload = json!({
        "name": format!("Limited Tester {}", unique_suffix),
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
    let (r_status, r_resp) = client.post_json("/v1/role", &role_payload).await;
    assert_eq!(r_status, StatusCode::CREATED);
    let role_id = read_body_json(r_resp).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    let email = format!("test_rbac_{}@email.com", unique_suffix);
    let user_payload = json!({
        "name": "Test User",
        "email": email,
        "password": "Password123!",
        "id_role": role_id
    });
    let (u_status, _) = client.post_json("/v1/user", &user_payload).await;
    assert_eq!(u_status, StatusCode::CREATED);

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

    test_rbac_forbidden_action(ctx, &limited_token).await;
    test_rbac_allowed_action(ctx, &limited_token).await;
    test_rbac_more_edge_cases(ctx, &limited_token, &role_id, &email).await;
}

async fn test_rbac_forbidden_action(ctx: &TestContext, token: &str) {
    let mut client = TestClient::new(ctx.router.clone());
    client.set_token(Some(token.to_string()));

    let unique_suffix = uuid::Uuid::new_v4().to_string()[..8].to_string();
    let payload = json!({
        "name": format!("Test Product {}", unique_suffix),
        "sku": format!("SKU-{}", unique_suffix),
        "category": "Test",
        "price": 10.5,
        "stock": 100,
        "description": "Test product description"
    });

    let (status, _) = client.post_json("/v1/product", &payload).await;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "RBAC failed to block unauthorized POST request"
    );
}

async fn test_rbac_allowed_action(ctx: &TestContext, token: &str) {
    let mut client = TestClient::new(ctx.router.clone());
    client.set_token(Some(token.to_string()));

    let (status, _) = client.get("/v1/product").await;
    assert_eq!(
        status,
        StatusCode::OK,
        "RBAC blocked an authorized GET request"
    );
}

async fn test_rbac_more_edge_cases(ctx: &TestContext, token: &str, role_id: &str, email: &str) {
    let mut client = TestClient::new(ctx.router.clone());
    client.set_token(Some(token.to_string()));

    let (del_status, _) = client.delete("/v1/product/some-uuid").await;
    assert_eq!(del_status, StatusCode::FORBIDDEN);

    let (no_map_status, _) = client.get("/v1/user").await;
    assert_eq!(no_map_status, StatusCode::FORBIDDEN);

    use backend_rust::models::user;
    use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
    if let Some(u) = user::Entity::find()
        .filter(user::Column::Email.eq(email))
        .one(&ctx.db)
        .await
        .unwrap()
    {
        let user_id = u.id.clone();
        let mut active_user: user::ActiveModel = u.into();
        active_user.active = Set(false);
        active_user.update(&ctx.db).await.unwrap();
        let _ = ctx
            .cache
            .delete_key(&format!("session:{}:permissions", user_id))
            .await;
    }

    let (inactive_user_status, _) = client.get("/v1/product").await;
    assert_eq!(inactive_user_status, StatusCode::FORBIDDEN);

    if let Some(u) = user::Entity::find()
        .filter(user::Column::Email.eq(email))
        .one(&ctx.db)
        .await
        .unwrap()
    {
        let user_id = u.id.clone();
        let mut active_user: user::ActiveModel = u.into();
        active_user.active = Set(true);
        active_user.update(&ctx.db).await.unwrap();
        let _ = ctx
            .cache
            .delete_key(&format!("session:{}:permissions", user_id))
            .await;
    }

    use backend_rust::models::role;
    if let Some(r) = role::Entity::find_by_id(role_id.to_string())
        .one(&ctx.db)
        .await
        .unwrap()
    {
        let mut active_role: role::ActiveModel = r.into();
        active_role.active = Set(false);
        active_role.update(&ctx.db).await.unwrap();
        if let Ok(users) = user::Entity::find()
            .filter(user::Column::IdRole.eq(role_id))
            .all(&ctx.db)
            .await
        {
            for u in users {
                let _ = ctx
                    .cache
                    .delete_key(&format!("session:{}:permissions", u.id))
                    .await;
            }
        }
    }

    let (inactive_role_status, _) = client.get("/v1/product").await;
    assert_eq!(inactive_role_status, StatusCode::FORBIDDEN);

    if let Some(r) = role::Entity::find_by_id(role_id.to_string())
        .one(&ctx.db)
        .await
        .unwrap()
    {
        let mut active_role: role::ActiveModel = r.into();
        active_role.active = Set(true);
        active_role.update(&ctx.db).await.unwrap();
        if let Ok(users) = user::Entity::find()
            .filter(user::Column::IdRole.eq(role_id))
            .all(&ctx.db)
            .await
        {
            for u in users {
                let _ = ctx
                    .cache
                    .delete_key(&format!("session:{}:permissions", u.id))
                    .await;
            }
        }
    }
}
