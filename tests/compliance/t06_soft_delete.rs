use crate::common::{read_body_json, TestClient, TestContext};
use axum::http::StatusCode;
use backend_rust::models::{auth, product, user};
use sea_orm::EntityTrait;
use serde_json::json;

pub async fn run(ctx: &TestContext) {
    println!("=== Running Soft Delete & Anonymization Tests ===");
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

    test_lgpd_user_anonymization(ctx, &mut client).await;
    test_soft_delete_behavior(ctx, &mut client).await;
}

async fn test_lgpd_user_anonymization(ctx: &TestContext, client: &mut TestClient) {
    let unique_suffix = uuid::Uuid::new_v4().to_string()[..8].to_string();
    let email = format!("lgpd_test_{}@email.com", unique_suffix);
    let payload = json!({
        "name": "LGPD User Name",
        "email": email,
        "password": "Password123!",
        "id_role": "administrator",
        "phone": "11999998888",
        "document": "12345678909"
    });

    let (c_status, c_resp) = client.post_json("/v1/user", &payload).await;
    assert_eq!(c_status, StatusCode::CREATED);
    let user_id = read_body_json(c_resp).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    let db_user_before = user::Entity::find_by_id(&user_id)
        .one(&ctx.db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(db_user_before.name, "LGPD User Name");
    assert_eq!(db_user_before.phone.as_deref().unwrap(), "11999998888");
    assert_eq!(db_user_before.document.as_deref().unwrap(), "12345678909");
    assert!(!db_user_before.is_deleted.unwrap());

    let (g_status_active, g_resp_active) = client.get(&format!("/v1/user/{}", user_id)).await;
    assert_eq!(g_status_active, StatusCode::OK);
    let u_data = read_body_json(g_resp_active).await;
    assert_eq!(u_data["id"].as_str().unwrap(), &user_id);

    let (d_status, _) = client.delete(&format!("/v1/user/{}", user_id)).await;
    assert_eq!(d_status, StatusCode::NO_CONTENT);

    let (g_status, _) = client.get(&format!("/v1/user/{}", user_id)).await;
    assert_eq!(g_status, StatusCode::NOT_FOUND);

    let db_user_after = user::Entity::find_by_id(&user_id)
        .one(&ctx.db)
        .await
        .unwrap()
        .unwrap();
    assert!(
        db_user_after.is_deleted.unwrap(),
        "is_deleted was not set to true"
    );
    assert!(
        db_user_after.deleted_at.is_some(),
        "deleted_at was not populated"
    );
    assert_eq!(
        db_user_after.name, "Deleted User",
        "Name was not anonymized to 'Deleted User'"
    );
    assert_eq!(
        db_user_after.phone.as_deref().unwrap(),
        "00000000000",
        "Phone was not anonymized"
    );
    assert_eq!(
        db_user_after.document.as_deref().unwrap(),
        "00000000000",
        "Document was not anonymized"
    );
    assert!(
        db_user_after.email.starts_with("deleted-anonymized-"),
        "Email was not anonymized: {}",
        db_user_after.email
    );

    if let Some(ref auth_id) = db_user_after.id_auth {
        let db_auth = auth::Entity::find_by_id(auth_id)
            .one(&ctx.db)
            .await
            .unwrap()
            .unwrap();
        assert!(!db_auth.active);
        assert!(db_auth.is_deleted.unwrap());
        assert!(db_auth.deleted_at.is_some());
        assert!(
            db_auth.password.is_none(),
            "Password hash was not cleared in Auth table"
        );
    }
}

async fn test_soft_delete_behavior(ctx: &TestContext, client: &mut TestClient) {
    let unique_suffix = uuid::Uuid::new_v4().to_string()[..8].to_string();
    let payload = json!({
        "name": format!("Del Test Product {}", unique_suffix),
        "sku": format!("SKU-DEL-{}", unique_suffix),
        "category": "SoftDeleteTest",
        "price": 99.99,
        "stock": 10,
        "description": "To be deleted"
    });

    let (c_status, c_resp) = client.post_json("/v1/product", &payload).await;
    assert_eq!(c_status, StatusCode::CREATED);
    let product_id = read_body_json(c_resp).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    let (d_status, _) = client.delete(&format!("/v1/product/{}", product_id)).await;
    assert_eq!(d_status, StatusCode::NO_CONTENT);

    let (g_status, _) = client.get(&format!("/v1/product/{}", product_id)).await;
    assert_eq!(g_status, StatusCode::NOT_FOUND);

    let (_, list_resp) = client.get("/v1/product?page=0&size=100").await;
    let items = read_body_json(list_resp).await["items"]
        .as_array()
        .unwrap()
        .clone();
    let ids: Vec<String> = items
        .iter()
        .map(|p| p["id"].as_str().unwrap().to_string())
        .collect();
    assert!(
        !ids.contains(&product_id),
        "Soft deleted product is still visible in active list"
    );

    let db_product = product::Entity::find_by_id(&product_id)
        .one(&ctx.db)
        .await
        .unwrap()
        .unwrap();
    assert!(
        db_product.is_deleted.unwrap(),
        "is_deleted was not set to true for product"
    );
    assert!(
        db_product.deleted_at.is_some(),
        "deleted_at was not populated for product"
    );
}
