use crate::common::{read_body_json, TestClient, TestContext};
use axum::http::StatusCode;
use backend_rust::models::{product, user};
use chrono::{Duration, FixedOffset, Utc};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde_json::json;

pub async fn run(ctx: &TestContext) {
    println!("=== Running Dashboard Statistics Tests ===");
    ctx.clear_database().await;
    ctx.reset_rate_limiter().await;

    backend_rust::infra::bootstrap::bootstrap_database(&ctx.db)
        .await
        .unwrap();

    let mut client = TestClient::new(ctx.router.clone());

    let (anon_status, _) = client.get("/v1/dashboard/stats").await;
    assert_eq!(anon_status, StatusCode::UNAUTHORIZED);

    let login_payload = json!({
        "email": "admin@email.com",
        "password": "admin@123"
    });
    let (_, login_resp) = client.post_json("/v1/auth/login", &login_payload).await;
    let admin_token = read_body_json(login_resp).await["token"]
        .as_str()
        .unwrap()
        .to_string();

    client.set_token(Some(admin_token.clone()));

    let unique_suffix = uuid::Uuid::new_v4().to_string()[..8].to_string();
    let role_payload = json!({
        "name": format!("Limited Tester {}", unique_suffix),
        "description": "Test Role",
        "permissions": [
            {
                "id_feature": "product",
                "create": true,
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

    let email = format!("tester_{}@email.com", unique_suffix);
    let user_payload = json!({
        "name": "Limited Tester User",
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
    let limited_token = read_body_json(login_resp_lim).await["token"]
        .as_str()
        .unwrap()
        .to_string();

    limited_client.set_token(Some(limited_token));

    let (forbidden_status, _) = limited_client.get("/v1/dashboard/stats").await;
    assert_eq!(forbidden_status, StatusCode::FORBIDDEN);
    let offset = FixedOffset::west_opt(3 * 3600).unwrap();
    let now = Utc::now();
    let ten_days_ago = now - Duration::days(10);
    let five_days_ago = now - Duration::days(5);

    let admin_user = user::Entity::find()
        .filter(user::Column::Email.eq("admin@email.com"))
        .one(&ctx.db)
        .await
        .unwrap()
        .unwrap();
    let mut admin_active: user::ActiveModel = admin_user.into();
    admin_active.created_at = Set(ten_days_ago.into());
    let admin_user = admin_active.update(&ctx.db).await.unwrap();

    let test_user = user::Entity::find_by_id(&user_id)
        .one(&ctx.db)
        .await
        .unwrap()
        .unwrap();
    let mut test_active: user::ActiveModel = test_user.into();
    test_active.created_at = Set(five_days_ago.into());
    let test_user = test_active.update(&ctx.db).await.unwrap();

    let product_1_id = uuid::Uuid::new_v4().to_string();
    let p1 = product::ActiveModel {
        id: Set(product_1_id),
        name: Set("P1".to_string()),
        sku: Set("SKU-P1".to_string()),
        category: Set("Cat1".to_string()),
        price: Set(sea_orm::prelude::Decimal::new(100, 0)),
        stock: Set(10),
        description: Set("D1".to_string()),
        active: Set(true),
        is_deleted: Set(Some(false)),
        deleted_at: Set(None),
        created_at: Set(five_days_ago.into()),
        updated_at: Set(five_days_ago.into()),
        id_user: Set(Some(admin_user.id.clone())),
    };
    p1.insert(&ctx.db).await.unwrap();

    let product_2_id = uuid::Uuid::new_v4().to_string();
    let p2 = product::ActiveModel {
        id: Set(product_2_id),
        name: Set("P2".to_string()),
        sku: Set("SKU-P2".to_string()),
        category: Set("Cat1".to_string()),
        price: Set(sea_orm::prelude::Decimal::new(200, 0)),
        stock: Set(20),
        description: Set("D2".to_string()),
        active: Set(true),
        is_deleted: Set(Some(false)),
        deleted_at: Set(None),
        created_at: Set(now.into()),
        updated_at: Set(now.into()),
        id_user: Set(Some(test_user.id.clone())),
    };
    p2.insert(&ctx.db).await.unwrap();

    let product_3_id = uuid::Uuid::new_v4().to_string();
    let p3 = product::ActiveModel {
        id: Set(product_3_id),
        name: Set("P3".to_string()),
        sku: Set("SKU-P3".to_string()),
        category: Set("Cat2".to_string()),
        price: Set(sea_orm::prelude::Decimal::new(300, 0)),
        stock: Set(30),
        description: Set("D3".to_string()),
        active: Set(true),
        is_deleted: Set(Some(false)),
        deleted_at: Set(None),
        created_at: Set(ten_days_ago.into()),
        updated_at: Set(ten_days_ago.into()),
        id_user: Set(Some(test_user.id.clone())),
    };
    p3.insert(&ctx.db).await.unwrap();

    let product_4_id = uuid::Uuid::new_v4().to_string();
    let p4 = product::ActiveModel {
        id: Set(product_4_id),
        name: Set("P4".to_string()),
        sku: Set("SKU-P4".to_string()),
        category: Set("Cat2".to_string()),
        price: Set(sea_orm::prelude::Decimal::new(400, 0)),
        stock: Set(40),
        description: Set("D4".to_string()),
        active: Set(true),
        is_deleted: Set(Some(false)),
        deleted_at: Set(None),
        created_at: Set(now.into()),
        updated_at: Set(now.into()),
        id_user: Set(None),
    };
    p4.insert(&ctx.db).await.unwrap();

    let (status, resp) = client.get("/v1/dashboard/stats").await;
    assert_eq!(status, StatusCode::OK);
    let body = read_body_json(resp).await;
    assert!(body["userCreationStats"].is_array());
    assert!(body["productCreationStats"].is_array());
    assert!(body["productsPerUser"].is_array());

    let user_stats = body["userCreationStats"].as_array().unwrap();
    let date_10_days_str = ten_days_ago
        .with_timezone(&offset)
        .format("%Y-%m-%d")
        .to_string();
    let date_5_days_str = five_days_ago
        .with_timezone(&offset)
        .format("%Y-%m-%d")
        .to_string();

    let stat_10_days = user_stats
        .iter()
        .find(|s| s["date"].as_str().unwrap() == date_10_days_str);
    assert!(stat_10_days.is_some());
    assert_eq!(stat_10_days.unwrap()["count"].as_i64().unwrap(), 1);

    let stat_5_days = user_stats
        .iter()
        .find(|s| s["date"].as_str().unwrap() == date_5_days_str);
    assert!(stat_5_days.is_some());
    assert_eq!(stat_5_days.unwrap()["count"].as_i64().unwrap(), 1);

    let product_stats = body["productCreationStats"].as_array().unwrap();
    let date_now_str = now.with_timezone(&offset).format("%Y-%m-%d").to_string();

    let p_stat_10_days = product_stats
        .iter()
        .find(|s| s["date"].as_str().unwrap() == date_10_days_str);
    assert!(p_stat_10_days.is_some());
    assert_eq!(p_stat_10_days.unwrap()["count"].as_i64().unwrap(), 1);

    let p_stat_5_days = product_stats
        .iter()
        .find(|s| s["date"].as_str().unwrap() == date_5_days_str);
    assert!(p_stat_5_days.is_some());
    assert_eq!(p_stat_5_days.unwrap()["count"].as_i64().unwrap(), 1);

    let p_stat_now = product_stats
        .iter()
        .find(|s| s["date"].as_str().unwrap() == date_now_str);
    assert!(p_stat_now.is_some());
    assert_eq!(p_stat_now.unwrap()["count"].as_i64().unwrap(), 2);

    let products_per_user = body["productsPerUser"].as_array().unwrap();
    let admin_stat = products_per_user
        .iter()
        .find(|u| u["userId"].as_str() == Some(&admin_user.id));
    assert!(admin_stat.is_some());
    assert_eq!(admin_stat.unwrap()["count"].as_i64().unwrap(), 1);
    assert_eq!(
        admin_stat.unwrap()["userName"].as_str().unwrap(),
        &admin_user.name
    );

    let test_stat = products_per_user
        .iter()
        .find(|u| u["userId"].as_str() == Some(&test_user.id));
    assert!(test_stat.is_some());
    assert_eq!(test_stat.unwrap()["count"].as_i64().unwrap(), 2);
    assert_eq!(
        test_stat.unwrap()["userName"].as_str().unwrap(),
        &test_user.name
    );

    let anon_stat = products_per_user.iter().find(|u| u["userId"].is_null());
    assert!(anon_stat.is_some());
    assert_eq!(anon_stat.unwrap()["count"].as_i64().unwrap(), 1);
    assert_eq!(
        anon_stat.unwrap()["userName"].as_str().unwrap(),
        "Anonymous"
    );

    let start_filter_str = (now.with_timezone(&offset) - Duration::days(6))
        .format("%Y-%m-%d")
        .to_string();
    let end_filter_str = (now.with_timezone(&offset) - Duration::days(2))
        .format("%Y-%m-%d")
        .to_string();
    let (status_filtered, resp_filtered) = client
        .get(&format!(
            "/v1/dashboard/stats?createdAt_start={}&createdAt_end={}",
            start_filter_str, end_filter_str
        ))
        .await;
    assert_eq!(status_filtered, StatusCode::OK);
    let body_filtered = read_body_json(resp_filtered).await;

    let u_filtered = body_filtered["userCreationStats"].as_array().unwrap();
    assert_eq!(u_filtered.len(), 1);
    assert_eq!(u_filtered[0]["date"].as_str().unwrap(), &date_5_days_str);
    assert_eq!(u_filtered[0]["count"].as_i64().unwrap(), 1);
    let p_filtered = body_filtered["productCreationStats"].as_array().unwrap();
    assert_eq!(p_filtered.len(), 1);
    assert_eq!(p_filtered[0]["date"].as_str().unwrap(), &date_5_days_str);
    assert_eq!(p_filtered[0]["count"].as_i64().unwrap(), 1);
    let ppu_filtered = body_filtered["productsPerUser"].as_array().unwrap();
    assert_eq!(ppu_filtered.len(), 1);
    assert_eq!(ppu_filtered[0]["userId"].as_str().unwrap(), &admin_user.id);
    assert_eq!(ppu_filtered[0]["count"].as_i64().unwrap(), 1);

    let (bad_status, _) = client
        .get("/v1/dashboard/stats?createdAt_start=2026-05-15&createdAt_end=2026-05-10")
        .await;
    assert_eq!(bad_status, StatusCode::BAD_REQUEST);
}
