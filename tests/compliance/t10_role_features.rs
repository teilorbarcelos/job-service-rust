use crate::common::{read_body_json, TestClient, TestContext};
use axum::http::StatusCode;
use serde_json::json;

pub async fn run(ctx: &TestContext) {
    println!("=== Running Role Features Tests ===");
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

    let unique_suffix = uuid::Uuid::new_v4().to_string()[..8].to_string();

    let allowed_role_payload = json!({
        "name": format!("Role View Allowed {}", unique_suffix),
        "description": "Allowed to view roles",
        "permissions": [
            { "id_feature": "role", "create": false, "view": true, "delete": false, "activate": false }
        ]
    });
    let (_, resp) = admin_client
        .post_json("/v1/role", &allowed_role_payload)
        .await;
    let allowed_role_id = read_body_json(resp).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    let allowed_user_email = format!("role_allowed_{}@email.com", unique_suffix);
    let allowed_user_payload = json!({
        "name": "Allowed Role Viewer",
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
        "name": format!("Role View Forbidden {}", unique_suffix),
        "description": "Forbidden to view roles",
        "permissions": [
            { "id_feature": "role", "create": false, "view": false, "delete": false, "activate": false }
        ]
    });
    let (_, resp) = admin_client
        .post_json("/v1/role", &forbidden_role_payload)
        .await;
    let forbidden_role_id = read_body_json(resp).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    let forbidden_user_email = format!("role_forbidden_{}@email.com", unique_suffix);
    let forbidden_user_payload = json!({
        "name": "Forbidden Role Viewer",
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

    test_list_role_features_by_admin(ctx, &mut admin_client).await;
    test_list_role_features_rbac_forbidden(ctx, &mut forbidden_client).await;
    test_list_role_features_rbac_allowed(ctx, &mut allowed_client).await;
    test_get_role_by_id_schema_compliance(ctx, &mut admin_client).await;
    test_update_role_permissions(ctx, &mut admin_client).await;
    test_list_roles_queries(ctx, &mut admin_client).await;
    test_role_edge_cases(ctx, &mut admin_client).await;

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

async fn test_list_role_features_by_admin(_ctx: &TestContext, client: &mut TestClient) {
    let (status, resp) = client.get("/v1/role/features").await;
    assert_eq!(status, StatusCode::OK);

    let items = read_body_json(resp).await;
    assert!(items.is_array());
    let list = items.as_array().unwrap();
    assert!(!list.is_empty());

    let f = &list[0];
    assert!(f.get("id").is_some());
    assert!(f.get("name").is_some());
}

async fn test_list_role_features_rbac_forbidden(_ctx: &TestContext, client: &mut TestClient) {
    let (status, _) = client.get("/v1/role/features").await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

async fn test_list_role_features_rbac_allowed(_ctx: &TestContext, client: &mut TestClient) {
    let (status, resp) = client.get("/v1/role/features").await;
    assert_eq!(status, StatusCode::OK);
    let items = read_body_json(resp).await;
    assert!(items.is_array());
}

async fn test_get_role_by_id_schema_compliance(_ctx: &TestContext, client: &mut TestClient) {
    let (status, resp) = client.get("/v1/role/administrator").await;
    assert_eq!(status, StatusCode::OK);

    let body = read_body_json(resp).await;
    assert_eq!(body["id"].as_str().unwrap(), "administrator");
    assert!(body.get("name").is_some());
    assert!(body.get("description").is_some());
    assert!(body.get("active").is_some());

    let permissions = body
        .get("RoleFeature")
        .expect("Missing RoleFeature array")
        .as_array()
        .unwrap();
    assert!(!permissions.is_empty());

    let p = &permissions[0];
    assert!(p.get("id_feature").is_some());
    assert!(p.get("create").is_some());
    assert!(p.get("view").is_some());
    assert!(p.get("activate").is_some());
    assert!(p.get("delete").is_some());
}

async fn test_update_role_permissions(_ctx: &TestContext, client: &mut TestClient) {
    let unique_suffix = uuid::Uuid::new_v4().to_string()[..8].to_string();
    let role_payload = json!({
        "name": format!("Role Update Test {}", unique_suffix),
        "description": "Initial description",
        "permissions": [
            { "id_feature": "role", "create": false, "view": true, "delete": false, "activate": false }
        ]
    });
    let (_, create_resp) = client.post_json("/v1/role", &role_payload).await;
    let role_id = read_body_json(create_resp).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    let update_payload = json!({
        "name": format!("Role Update Test {}", unique_suffix),
        "description": "Updated description",
        "permissions": [
            { "id_feature": "role", "create": true, "view": true, "delete": true, "activate": true }
        ]
    });

    let (status, update_resp) = client
        .put_json(&format!("/v1/role/{}", role_id), &update_payload)
        .await;
    assert_eq!(status, StatusCode::OK);
    let body = read_body_json(update_resp).await;
    assert_eq!(body["description"].as_str().unwrap(), "Updated description");

    let permissions = body["RoleFeature"].as_array().unwrap();
    assert_eq!(permissions.len(), 1);
    assert!(permissions[0]["create"].as_bool().unwrap());

    let update_no_perms_payload = json!({
        "name": format!("Role Update Test {}", unique_suffix),
        "description": "Updated description again"
    });
    let (status2, update_resp2) = client
        .put_json(&format!("/v1/role/{}", role_id), &update_no_perms_payload)
        .await;
    assert_eq!(status2, StatusCode::OK);
    let body2 = read_body_json(update_resp2).await;
    let permissions2 = body2["RoleFeature"].as_array().unwrap();
    assert_eq!(permissions2.len(), 1);

    let _ = client.delete(&format!("/v1/role/{}", role_id)).await;
}

async fn test_list_roles_queries(_ctx: &TestContext, client: &mut TestClient) {
    let (status1, resp1) = client.get("/v1/role").await;
    assert_eq!(status1, StatusCode::OK);
    let body1 = read_body_json(resp1).await;
    assert!(body1["items"].is_array());
    assert!(body1["total"].as_u64().is_some());

    let (status2, resp2) = client.get("/v1/role/all").await;
    assert_eq!(status2, StatusCode::OK);
    let body2 = read_body_json(resp2).await;
    assert!(body2["items"].is_array());
}

async fn test_role_edge_cases(_ctx: &TestContext, client: &mut TestClient) {
    let empty_name_payload = json!({
        "name": "",
        "description": "Empty name role",
        "permissions": []
    });
    let (status1, resp1) = client.post_json("/v1/role", &empty_name_payload).await;
    assert_eq!(status1, StatusCode::CREATED);
    let empty_role_id = read_body_json(resp1).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    let conflict_payload = json!({
        "name": "FORCE_CONFLICT_ROLE",
        "description": "Forced conflict role",
        "permissions": []
    });
    let (status2, resp2) = client.post_json("/v1/role", &conflict_payload).await;
    assert_eq!(status2, StatusCode::CREATED);
    let conflict_role_id = read_body_json(resp2).await["id"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(conflict_role_id, "forced-conflict-id");

    let (status3, resp3) = client.post_json("/v1/role", &conflict_payload).await;
    assert_eq!(status3, StatusCode::CONFLICT);
    let err_body = read_body_json(resp3).await;
    assert_eq!(
        err_body["message"].as_str().unwrap(),
        "Perfil com ID ou nome correspondente já cadastrado"
    );

    let _ = client.delete(&format!("/v1/role/{}", empty_role_id)).await;
    let _ = client
        .delete(&format!("/v1/role/{}", conflict_role_id))
        .await;
}
