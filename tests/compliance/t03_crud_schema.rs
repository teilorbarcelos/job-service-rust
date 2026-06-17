use crate::common::{read_body_json, TestClient, TestContext};
use axum::http::StatusCode;
use serde_json::json;

pub async fn run(ctx: &TestContext) {
    println!("=== Running CRUD Schema Tests ===");
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

    test_schema_missing_required_field(ctx, &mut client).await;
    test_schema_unknown_field_rejection(ctx, &mut client).await;
    test_create_user_with_invalid_role(ctx, &mut client).await;
    test_app_json_rejections(ctx, &mut client).await;
    test_user_update_edge_cases(ctx, &mut client).await;
    test_create_user_email_conflict(ctx, &mut client).await;
    test_product_crud_edge_cases(ctx, &mut client).await;
}

async fn test_schema_missing_required_field(_ctx: &TestContext, client: &mut TestClient) {
    let payload = json!({
        "description": "Missing name role",
        "permissions": []
    });
    let (status, resp) = client.post_json("/v1/role", &payload).await;

    assert!(
        status == StatusCode::BAD_REQUEST || status == StatusCode::UNPROCESSABLE_ENTITY,
        "Backend allowed payload with missing required fields: {:?}",
        status
    );

    let data = read_body_json(resp).await;
    assert!(
        data.get("message").is_some()
            || data.get("errors").is_some()
            || data.get("error").is_some(),
        "Expected error detail in response body: {:?}",
        data
    );
}

async fn test_schema_unknown_field_rejection(_ctx: &TestContext, client: &mut TestClient) {
    let unique_suffix = uuid::Uuid::new_v4().to_string()[..8].to_string();
    let payload = json!({
        "name": format!("Strict Role {}", unique_suffix),
        "description": "Role to test strict schema",
        "permissions": [],
        "hacker_field": "This should be rejected or stripped"
    });
    let (status, resp) = client.post_json("/v1/role", &payload).await;

    assert!(
        status == StatusCode::OK
            || status == StatusCode::CREATED
            || status == StatusCode::BAD_REQUEST
            || status == StatusCode::UNPROCESSABLE_ENTITY,
        "Unexpected status for unknown field payload: {:?}",
        status
    );

    if status == StatusCode::OK || status == StatusCode::CREATED {
        let data = read_body_json(resp).await;
        assert!(
            data.get("hacker_field").is_none(),
            "Backend saved/returned unknown field (Mass Assignment Vulnerability): {:?}",
            data
        );
    }
}

async fn test_create_user_with_invalid_role(_ctx: &TestContext, client: &mut TestClient) {
    let payload = json!({
        "name": "Test User Invalid Role",
        "email": "invalid_role_user@email.com",
        "password": "Password123!",
        "id_role": "non-existent-role-uuid"
    });
    let (status, resp) = client.post_json("/v1/user", &payload).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let body = read_body_json(resp).await;
    assert!(body["message"]
        .as_str()
        .unwrap()
        .contains("ID de perfil (role) fornecido inválido"));
}

async fn test_app_json_rejections(_ctx: &TestContext, client: &mut TestClient) {
    let (status1, resp1) = client
        .request_with_headers(
            "POST",
            "/v1/role",
            axum::body::Body::from("{}"),
            None,
            vec![],
        )
        .await;
    assert_eq!(status1, StatusCode::BAD_REQUEST);
    let body1 = read_body_json(resp1).await;
    println!("test_app_json_rejections body1: {:?}", body1);
    assert!(body1["message"]
        .as_str()
        .unwrap()
        .contains("Cabeçalho Content-Type esperado"));

    let (status2, resp2) = client
        .request_with_headers(
            "POST",
            "/v1/role",
            axum::body::Body::from("{invalid"),
            Some("application/json"),
            vec![],
        )
        .await;
    assert_eq!(status2, StatusCode::BAD_REQUEST);
    let body2 = read_body_json(resp2).await;
    assert!(body2["message"]
        .as_str()
        .unwrap()
        .contains("Erro de sintaxe no JSON"));

    let (status3, resp3) = client
        .request_with_headers(
            "POST",
            "/v1/role",
            axum::body::Body::from("123"),
            Some("application/json"),
            vec![],
        )
        .await;
    assert_eq!(status3, StatusCode::BAD_REQUEST);
    let body3 = read_body_json(resp3).await;
    assert!(
        body3["message"]
            .as_str()
            .unwrap()
            .contains("Erro de validação do JSON")
            || body3["message"].as_str().unwrap().contains("desserializar")
    );
}

async fn test_user_update_edge_cases(_ctx: &TestContext, client: &mut TestClient) {
    let suffix = uuid::Uuid::new_v4().to_string()[..8].to_string();

    let email_a = format!("user_a_{}@email.com", suffix);
    let payload_a = json!({
        "name": "User A",
        "email": email_a,
        "password": "Password123!",
        "id_role": "administrator"
    });
    let (_, resp_a) = client.post_json("/v1/user", &payload_a).await;
    let user_a_id = read_body_json(resp_a).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    let email_b = format!("user_b_{}@email.com", suffix);
    let payload_b = json!({
        "name": "User B",
        "email": email_b,
        "password": "Password123!",
        "id_role": "administrator"
    });
    let (_, resp_b) = client.post_json("/v1/user", &payload_b).await;
    let user_b_id = read_body_json(resp_b).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    let conflict_update = json!({
        "name": "User A Updated",
        "email": email_b,
        "id_role": "administrator",
        "active": true
    });
    let (status_c, _) = client
        .put_json(&format!("/v1/user/{}", user_a_id), &conflict_update)
        .await;
    assert_eq!(status_c, StatusCode::CONFLICT);

    let invalid_role_update = json!({
        "name": "User A Updated",
        "email": email_a,
        "id_role": "non-existent-role-uuid",
        "active": true
    });
    let (status_r, _) = client
        .put_json(&format!("/v1/user/{}", user_a_id), &invalid_role_update)
        .await;
    assert_eq!(status_r, StatusCode::BAD_REQUEST);

    let success_update = json!({
        "name": "User A Updated",
        "email": email_a,
        "id_role": "administrator",
        "active": false
    });
    let (status_s, resp_s) = client
        .put_json(&format!("/v1/user/{}", user_a_id), &success_update)
        .await;
    assert_eq!(status_s, StatusCode::OK);
    let body_s = read_body_json(resp_s).await;
    assert!(!body_s["active"].as_bool().unwrap());

    let _ = client.delete(&format!("/v1/user/{}", user_a_id)).await;
    let _ = client.delete(&format!("/v1/user/{}", user_b_id)).await;
}

async fn test_create_user_email_conflict(_ctx: &TestContext, client: &mut TestClient) {
    let suffix = uuid::Uuid::new_v4().to_string()[..8].to_string();
    let email = format!("conflict_user_{}@email.com", suffix);
    let payload = json!({
        "name": "Conflict User 1",
        "email": email.clone(),
        "password": "Password123!",
        "id_role": "administrator"
    });

    let (status1, _) = client.post_json("/v1/user", &payload).await;
    assert_eq!(status1, StatusCode::CREATED);

    let payload2 = json!({
        "name": "Conflict User 2",
        "email": email,
        "password": "Password123!",
        "id_role": "administrator"
    });
    let (status2, resp2) = client.post_json("/v1/user", &payload2).await;
    assert_eq!(status2, StatusCode::CONFLICT);
    let body = read_body_json(resp2).await;
    assert!(body["message"]
        .as_str()
        .unwrap()
        .contains("E-mail já cadastrado no sistema"));
}

async fn test_product_crud_edge_cases(_ctx: &TestContext, client: &mut TestClient) {
    let suffix = uuid::Uuid::new_v4().to_string()[..8].to_string();
    let sku_a = format!("SKU-A-{}", suffix);
    let sku_b = format!("SKU-B-{}", suffix);

    let payload_a = json!({
        "name": "Product A",
        "sku": sku_a.clone(),
        "category": "Cat A",
        "price": 100.0,
        "stock": 10,
        "description": "Desc A"
    });
    let (status_c1, resp_c1) = client.post_json("/v1/product", &payload_a).await;
    assert_eq!(status_c1, StatusCode::CREATED);
    let prod_a_id = read_body_json(resp_c1).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    let payload_b = json!({
        "name": "Product B",
        "sku": sku_a.clone(),
        "category": "Cat B",
        "price": 150.0,
        "stock": 5,
        "description": "Desc B"
    });
    let (status_c2, resp_c2) = client.post_json("/v1/product", &payload_b).await;
    assert_eq!(status_c2, StatusCode::CONFLICT);
    let body_c2 = read_body_json(resp_c2).await;
    assert!(body_c2["message"]
        .as_str()
        .unwrap()
        .contains("SKU já cadastrado no sistema"));

    let payload_b_ok = json!({
        "name": "Product B",
        "sku": sku_b.clone(),
        "category": "Cat B",
        "price": 150.0,
        "stock": 5,
        "description": "Desc B"
    });
    let (status_c3, resp_c3) = client.post_json("/v1/product", &payload_b_ok).await;
    assert_eq!(status_c3, StatusCode::CREATED);
    let prod_b_id = read_body_json(resp_c3).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    let (status_g, resp_g) = client.get(&format!("/v1/product/{}", prod_a_id)).await;
    assert_eq!(status_g, StatusCode::OK);
    let body_g = read_body_json(resp_g).await;
    assert_eq!(body_g["sku"].as_str().unwrap(), &sku_a);

    let update_non_existent = json!({
        "name": "Ghost",
        "sku": "SKU-GHOST",
        "category": "Ghost",
        "price": 0.0,
        "stock": 0,
        "description": "Ghost",
        "active": true
    });
    let (status_u1, _) = client
        .put_json("/v1/product/non-existent-id", &update_non_existent)
        .await;
    assert_eq!(status_u1, StatusCode::NOT_FOUND);

    let update_conflict = json!({
        "name": "Product A Updated",
        "sku": sku_b.clone(),
        "category": "Cat A",
        "price": 120.0,
        "stock": 12,
        "description": "Desc A",
        "active": true
    });
    let (status_u2, resp_u2) = client
        .put_json(&format!("/v1/product/{}", prod_a_id), &update_conflict)
        .await;
    assert_eq!(status_u2, StatusCode::CONFLICT);
    let body_u2 = read_body_json(resp_u2).await;
    assert!(body_u2["message"]
        .as_str()
        .unwrap()
        .contains("SKU já está sendo utilizado por outro produto"));

    let update_ok = json!({
        "name": "Product A Updated",
        "sku": sku_a.clone(),
        "category": "Cat A",
        "price": 120.0,
        "stock": 12,
        "description": "Desc A Updated",
        "active": false
    });
    let (status_u3, resp_u3) = client
        .put_json(&format!("/v1/product/{}", prod_a_id), &update_ok)
        .await;
    assert_eq!(status_u3, StatusCode::OK);
    let body_u3 = read_body_json(resp_u3).await;
    assert_eq!(body_u3["name"].as_str().unwrap(), "Product A Updated");
    assert!(!body_u3["active"].as_bool().unwrap());

    let (status_all, _) = client.get("/v1/product/all").await;
    assert_eq!(status_all, StatusCode::OK);

    let _ = client.delete(&format!("/v1/product/{}", prod_a_id)).await;
    let _ = client.delete(&format!("/v1/product/{}", prod_b_id)).await;
}
