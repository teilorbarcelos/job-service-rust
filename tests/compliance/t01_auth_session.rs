use crate::common::{read_body_json, TestClient, TestContext};
use axum::http::StatusCode;
use sea_orm::ConnectionTrait;
use serde_json::json;

pub async fn run(ctx: &TestContext) {
    println!("=== Running Auth & Session Tests ===");
    ctx.clear_database().await;
    ctx.reset_rate_limiter().await;

    test_login_invalid_credentials(ctx).await;
    test_login_success(ctx).await;
    test_redis_session_created(ctx).await;
    test_refresh_token(ctx).await;
    test_invalid_tokens_return_unauthorized_error(ctx).await;
    test_get_me_structure(ctx).await;
    test_session_invalidation_on_mutation(ctx).await;
    test_login_inactive_user(ctx).await;
    test_login_inactive_role(ctx).await;
    test_logout(ctx).await;
    test_login_wrong_password(ctx).await;
    test_login_inactive_auth(ctx).await;
    test_refresh_token_errors(ctx).await;
    test_login_role_not_found(ctx).await;
}

async fn test_login_invalid_credentials(ctx: &TestContext) {
    let mut client = TestClient::new(ctx.router.clone());
    let payload = json!({
        "email": "invalid@example.com",
        "password": "wrong"
    });
    let (status, _) = client.post_json("/v1/auth/login", &payload).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

async fn test_login_success(ctx: &TestContext) {
    let mut client = TestClient::new(ctx.router.clone());
    let payload = json!({
        "email": "admin@email.com",
        "password": "admin@123"
    });
    let (status, resp) = client.post_json("/v1/auth/login", &payload).await;
    assert_eq!(status, StatusCode::OK);

    let body = read_body_json(resp).await;
    assert!(body.get("token").is_some());
    assert!(body.get("refreshToken").is_some());

    let user = body.get("user").expect("Missing user object");
    assert!(user.get("id").is_some());
    assert_eq!(
        user.get("email").unwrap().as_str().unwrap(),
        "admin@email.com"
    );

    let role = user.get("role").expect("Missing role object");
    assert!(role.get("permissions").is_some());
}

async fn test_redis_session_created(ctx: &TestContext) {
    let mut client = TestClient::new(ctx.router.clone());
    let payload = json!({
        "email": "admin@email.com",
        "password": "admin@123"
    });
    let (status, resp) = client.post_json("/v1/auth/login", &payload).await;
    assert_eq!(status, StatusCode::OK);

    let body = read_body_json(resp).await;
    let user_id = body["user"]["id"].as_str().unwrap();

    let mut conn = ctx
        .cache
        .pool
        .get()
        .await
        .expect("Failed to get redis connection");
    let keys: Vec<String> = redis::cmd("KEYS")
        .arg(format!("*{}*", user_id))
        .query_async(&mut conn)
        .await
        .unwrap_or_default();

    assert!(
        !keys.is_empty(),
        "No Redis session found for logged in user"
    );
}

async fn test_refresh_token(ctx: &TestContext) {
    let mut client = TestClient::new(ctx.router.clone());
    let payload = json!({
        "email": "admin@email.com",
        "password": "admin@123"
    });
    let (status, resp) = client.post_json("/v1/auth/login", &payload).await;
    assert_eq!(status, StatusCode::OK);

    let body = read_body_json(resp).await;
    let refresh_token = body["refreshToken"].as_str().unwrap();

    let refresh_payload = json!({
        "refreshToken": refresh_token
    });
    let (r_status, r_resp) = client.post_json("/v1/auth/refresh", &refresh_payload).await;
    assert_eq!(r_status, StatusCode::OK);

    let new_data = read_body_json(r_resp).await;
    assert!(new_data.get("token").is_some());
    assert!(new_data.get("refreshToken").is_some());

    let new_token = new_data["token"].as_str().unwrap();
    assert_eq!(
        new_token.split('.').count(),
        3,
        "Expected access token to be a signed JWT"
    );

    let user = new_data.get("user").expect("Missing user object");
    assert!(user.get("id").is_some());
    assert_eq!(
        user.get("email").unwrap().as_str().unwrap(),
        "admin@email.com"
    );
    assert!(user.get("role").is_some());
}

async fn test_invalid_tokens_return_unauthorized_error(ctx: &TestContext) {
    let mut client = TestClient::new(ctx.router.clone());

    client.set_token(Some("invalid-token-signature".to_string()));
    let (status, resp) = client.get("/v1/auth/me").await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let body = read_body_json(resp).await;
    assert_eq!(body["error"].as_str().unwrap(), "UnauthorizedError");

    let mut custom_client = TestClient::new(ctx.router.clone());
    let (status_non_bearer, resp_non_bearer) = custom_client
        .request_with_headers(
            "GET",
            "/v1/auth/me",
            axum::body::Body::empty(),
            None,
            vec![("Authorization", "Basic abc")],
        )
        .await;
    assert_eq!(status_non_bearer, StatusCode::UNAUTHORIZED);
    let body_non_bearer = read_body_json(resp_non_bearer).await;
    assert_eq!(
        body_non_bearer["error"].as_str().unwrap(),
        "UnauthorizedError"
    );
    assert_eq!(
        body_non_bearer["message"].as_str().unwrap(),
        "Token deve ser do tipo Bearer"
    );

    client.set_token(None);
    let refresh_payload = json!({
        "refreshToken": "invalid-refresh-token"
    });
    let (r_status, r_resp) = client.post_json("/v1/auth/refresh", &refresh_payload).await;
    assert_eq!(r_status, StatusCode::UNAUTHORIZED);

    let r_body = read_body_json(r_resp).await;
    assert_eq!(r_body["error"].as_str().unwrap(), "UnauthorizedError");
}

async fn test_get_me_structure(ctx: &TestContext) {
    let mut client = TestClient::new(ctx.router.clone());

    let payload = json!({
        "email": "admin@email.com",
        "password": "admin@123"
    });
    let (_, login_resp) = client.post_json("/v1/auth/login", &payload).await;
    let login_body = read_body_json(login_resp).await;
    let token = login_body["token"].as_str().unwrap().to_string();

    client.set_token(Some(token));
    let (status, resp) = client.get("/v1/auth/me").await;
    assert_eq!(status, StatusCode::OK);

    let body = read_body_json(resp).await;
    assert!(body.get("user").is_some());
    let user = &body["user"];
    assert!(user.get("email").is_some());
    assert!(user.get("role").is_some());
}

async fn test_session_invalidation_on_mutation(ctx: &TestContext) {
    let mut client = TestClient::new(ctx.router.clone());

    let login_payload = json!({
        "email": "admin@email.com",
        "password": "admin@123"
    });
    let (_, login_resp) = client.post_json("/v1/auth/login", &login_payload).await;
    let login_body = read_body_json(login_resp).await;
    let token = login_body["token"].as_str().unwrap().to_string();
    let user_id = login_body["user"]["id"].as_str().unwrap().to_string();
    let role_id = login_body["user"]["role"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let name = login_body["user"]["name"].as_str().unwrap().to_string();

    client.set_token(Some(token.clone()));

    let update_payload = json!({
        "name": format!("{} Updated", name),
        "email": "admin@email.com",
        "id_role": role_id,
        "phone": "11999999999",
        "document": "00000000000"
    });
    let (up_status, _) = client
        .put_json(&format!("/v1/user/{}", user_id), &update_payload)
        .await;
    assert_eq!(up_status, StatusCode::OK);

    let (me_status, _) = client.get("/v1/auth/me").await;
    assert_eq!(
        me_status,
        StatusCode::UNAUTHORIZED,
        "Session was not invalidated after user mutation"
    );
}

async fn test_login_inactive_user(ctx: &TestContext) {
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
    let test_email = format!("inactive_user_{}@email.com", unique_suffix);
    let test_password = format!("Pass_{}!", unique_suffix);

    let create_payload = json!({
        "name": "Inactive User Test",
        "email": test_email,
        "password": test_password,
        "id_role": "administrator"
    });
    let (c_status, c_resp) = client.post_json("/v1/user", &create_payload).await;
    assert_eq!(c_status, StatusCode::CREATED);

    let user_id = read_body_json(c_resp).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    let patch_payload = json!({
        "active": false
    });
    let (p_status, _) = client
        .patch_json(&format!("/v1/user/{}/status", user_id), &patch_payload)
        .await;
    assert_eq!(p_status, StatusCode::OK);

    client.set_token(None);
    let deact_login_payload = json!({
        "email": test_email,
        "password": test_password
    });
    let (login_status, _) = client
        .post_json("/v1/auth/login", &deact_login_payload)
        .await;
    assert!(
        login_status == StatusCode::UNAUTHORIZED || login_status == StatusCode::FORBIDDEN,
        "Login for inactive user should be prevented, got {:?}",
        login_status
    );
}

async fn test_login_inactive_role(ctx: &TestContext) {
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
    let test_email = format!("inactive_role_{}@email.com", unique_suffix);
    let test_password = format!("Pass_{}!", unique_suffix);

    let role_payload = json!({
        "name": format!("Temp Role {}", unique_suffix),
        "description": "Will be deactivated",
        "permissions": []
    });
    let (r_status, r_resp) = client.post_json("/v1/role", &role_payload).await;
    assert_eq!(r_status, StatusCode::CREATED);
    let role_id = read_body_json(r_resp).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    let user_payload = json!({
        "name": "Inactive Role Test User",
        "email": test_email,
        "password": test_password,
        "id_role": role_id
    });
    let (u_status, _) = client.post_json("/v1/user", &user_payload).await;
    assert_eq!(u_status, StatusCode::CREATED);

    let patch_payload = json!({
        "active": false
    });
    let (p_status, _) = client
        .patch_json(&format!("/v1/role/{}/status", role_id), &patch_payload)
        .await;
    assert_eq!(p_status, StatusCode::OK);

    client.set_token(None);
    let role_login_payload = json!({
        "email": test_email,
        "password": test_password
    });
    let (login_status, _) = client
        .post_json("/v1/auth/login", &role_login_payload)
        .await;
    assert!(
        login_status == StatusCode::UNAUTHORIZED || login_status == StatusCode::FORBIDDEN,
        "Login for user with inactive role should be prevented"
    );
}

async fn test_logout(ctx: &TestContext) {
    let mut client = TestClient::new(ctx.router.clone());
    let payload = json!({
        "email": "admin@email.com",
        "password": "admin@123"
    });
    let (_, login_resp) = client.post_json("/v1/auth/login", &payload).await;
    let token = read_body_json(login_resp).await["token"]
        .as_str()
        .unwrap()
        .to_string();

    client.set_token(Some(token));
    let (status, resp) = client.post_json("/v1/auth/logout", &json!({})).await;
    assert_eq!(status, StatusCode::OK);
    let body = read_body_json(resp).await;
    assert!(body["status"].as_bool().unwrap());

    let (me_status, _) = client.get("/v1/auth/me").await;
    assert_eq!(me_status, StatusCode::UNAUTHORIZED);
}

async fn test_login_wrong_password(ctx: &TestContext) {
    let mut client = TestClient::new(ctx.router.clone());
    let payload = json!({
        "email": "admin@email.com",
        "password": "wrong"
    });
    let (status, _) = client.post_json("/v1/auth/login", &payload).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

async fn test_login_inactive_auth(ctx: &TestContext) {
    let mut client = TestClient::new(ctx.router.clone());

    use backend_rust::models::auth;
    use sea_orm::{ActiveModelTrait, EntityTrait, Set};

    if let Some(a) = auth::Entity::find_by_id("auth-admin-uuid-00000000000000000001".to_string())
        .one(&ctx.db)
        .await
        .unwrap()
    {
        let mut active_auth: auth::ActiveModel = a.into();
        active_auth.active = Set(false);
        active_auth.update(&ctx.db).await.unwrap();
    }

    let payload = json!({
        "email": "admin@email.com",
        "password": "admin@123"
    });
    let (status, _) = client.post_json("/v1/auth/login", &payload).await;
    assert_eq!(status, StatusCode::FORBIDDEN);

    if let Some(a) = auth::Entity::find_by_id("auth-admin-uuid-00000000000000000001".to_string())
        .one(&ctx.db)
        .await
        .unwrap()
    {
        let mut active_auth: auth::ActiveModel = a.into();
        active_auth.active = Set(true);
        active_auth.update(&ctx.db).await.unwrap();
    }
}

async fn test_refresh_token_errors(ctx: &TestContext) {
    let mut client = TestClient::new(ctx.router.clone());

    use backend_rust::infra::auth::AuthService;
    use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};

    let (_, refresh_token) = AuthService::generate_tokens(
        "admin-user-uuid",
        "admin@email.com",
        "administrator",
        &ctx.config.jwt_secret,
        3600,
    )
    .unwrap();

    let (status, _) = client
        .post_json(
            "/v1/auth/refresh",
            &json!({ "refreshToken": refresh_token }),
        )
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let payload = json!({
        "email": "admin@email.com",
        "password": "admin@123"
    });
    let (_, login_resp) = client.post_json("/v1/auth/login", &payload).await;
    let body = read_body_json(login_resp).await;
    let valid_refresh_token = body["refreshToken"].as_str().unwrap().to_string();

    use backend_rust::models::user;
    if let Some(u) = user::Entity::find()
        .filter(user::Column::Email.eq("admin@email.com"))
        .one(&ctx.db)
        .await
        .unwrap()
    {
        let mut active_user: user::ActiveModel = u.into();
        active_user.active = Set(false);
        active_user.update(&ctx.db).await.unwrap();
    }

    let (status, _) = client
        .post_json(
            "/v1/auth/refresh",
            &json!({ "refreshToken": valid_refresh_token }),
        )
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    if let Some(u) = user::Entity::find()
        .filter(user::Column::Email.eq("admin@email.com"))
        .one(&ctx.db)
        .await
        .unwrap()
    {
        let mut active_user: user::ActiveModel = u.into();
        active_user.active = Set(true);
        active_user.update(&ctx.db).await.unwrap();
    }

    let (_, login_resp) = client.post_json("/v1/auth/login", &payload).await;
    let body = read_body_json(login_resp).await;
    let valid_refresh_token = body["refreshToken"].as_str().unwrap().to_string();

    use backend_rust::models::role;
    if let Some(r) = role::Entity::find_by_id("administrator".to_string())
        .one(&ctx.db)
        .await
        .unwrap()
    {
        let mut active_role: role::ActiveModel = r.into();
        active_role.active = Set(false);
        active_role.update(&ctx.db).await.unwrap();
    }

    let (status, _) = client
        .post_json(
            "/v1/auth/refresh",
            &json!({ "refreshToken": valid_refresh_token }),
        )
        .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    if let Some(r) = role::Entity::find_by_id("administrator".to_string())
        .one(&ctx.db)
        .await
        .unwrap()
    {
        let mut active_role: role::ActiveModel = r.into();
        active_role.active = Set(true);
        active_role.update(&ctx.db).await.unwrap();
    }
}

async fn test_login_role_not_found(ctx: &TestContext) {
    let mut client = TestClient::new(ctx.router.clone());

    use backend_rust::models::{auth, user};
    use sea_orm::{ActiveModelTrait, EntityTrait, Set, Statement};

    let auth_id = format!("a-{}", uuid::Uuid::new_v4());
    let password_hash = bcrypt::hash("password123", bcrypt::DEFAULT_COST).unwrap();
    let temp_auth = auth::ActiveModel {
        id: Set(auth_id.clone()),
        password: Set(Some(password_hash)),
        active: Set(true),
        created_at: Set(chrono::Utc::now().into()),
        updated_at: Set(chrono::Utc::now().into()),
        ..Default::default()
    };
    temp_auth.insert(&ctx.db).await.unwrap();

    let _ = ctx
        .db
        .execute(Statement::from_string(
            ctx.db.get_database_backend(),
            "ALTER TABLE \"User\" DISABLE TRIGGER ALL".to_string(),
        ))
        .await;

    let user_id = format!("u-{}", uuid::Uuid::new_v4());
    let email = format!("{}@test.com", user_id);
    let temp_user = user::ActiveModel {
        id: Set(user_id.clone()),
        name: Set("Temp User".to_string()),
        email: Set(email.clone()),
        id_role: Set("non-existent-role".to_string()),
        id_auth: Set(Some(auth_id.clone())),
        active: Set(true),
        is_deleted: Set(Some(false)),
        deleted_at: Set(None),
        created_at: Set(chrono::Utc::now().into()),
        updated_at: Set(chrono::Utc::now().into()),
        ..Default::default()
    };
    temp_user.insert(&ctx.db).await.unwrap();

    let _ = ctx
        .db
        .execute(Statement::from_string(
            ctx.db.get_database_backend(),
            "ALTER TABLE \"User\" ENABLE TRIGGER ALL".to_string(),
        ))
        .await;

    let payload = json!({
        "email": email.clone(),
        "password": "password123"
    });

    let (status, _) = client.post_json("/v1/auth/login", &payload).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);

    let _ = ctx
        .db
        .execute(Statement::from_string(
            ctx.db.get_database_backend(),
            "ALTER TABLE \"User\" DISABLE TRIGGER ALL".to_string(),
        ))
        .await;

    let _ = user::Entity::delete_by_id(&user_id).exec(&ctx.db).await;

    let _ = ctx
        .db
        .execute(Statement::from_string(
            ctx.db.get_database_backend(),
            "ALTER TABLE \"User\" ENABLE TRIGGER ALL".to_string(),
        ))
        .await;

    let _ = auth::Entity::delete_by_id(&auth_id).exec(&ctx.db).await;
}
