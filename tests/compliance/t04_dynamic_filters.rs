use crate::common::{read_body_json, TestClient, TestContext};
use axum::http::StatusCode;
use serde_json::json;

pub async fn run(ctx: &TestContext) {
    println!("=== Running Dynamic Filters Tests ===");
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

    test_dynamic_filter_success(ctx, &mut client).await;
    test_dynamic_filter_missing_search_fields(ctx, &mut client).await;
    test_dynamic_filter_unmapped_search_field(ctx, &mut client).await;
    test_dynamic_filter_unallowed_filter_key(ctx, &mut client).await;
    test_dynamic_filter_invalid_date_format(ctx, &mut client).await;
    test_dynamic_filter_date_range(ctx, &mut client).await;
    test_dynamic_filter_active_status(ctx, &mut client).await;
    test_pagination_size_limit(ctx, &mut client).await;
    test_listing_sorting(ctx, &mut client).await;
}

async fn test_dynamic_filter_success(_ctx: &TestContext, client: &mut TestClient) {
    let url = "/v1/user/all?page=0&size=25&searchWord=Admin&searchFields=name,email,Role.name&orderBy=name&orderDirection=asc";
    let (status, resp) = client.get(url).await;
    assert_eq!(status, StatusCode::OK);

    let data = read_body_json(resp).await;
    assert!(data.get("items").is_some());
    assert!(data.get("total").is_some());
    assert!(data.get("page").is_some());
}

async fn test_dynamic_filter_missing_search_fields(_ctx: &TestContext, client: &mut TestClient) {
    let url = "/v1/user/all?page=0&size=25&searchWord=Admin&orderBy=name&orderDirection=asc";
    let (status, _) = client.get(url).await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "Backend allowed searchWord without searchFields"
    );
}

async fn test_dynamic_filter_unmapped_search_field(_ctx: &TestContext, client: &mut TestClient) {
    let url = "/v1/user/all?page=0&size=25&searchWord=Admin&searchFields=password&orderBy=name&orderDirection=asc";
    let (status, _) = client.get(url).await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "Backend allowed search on unmapped/unauthorized fields"
    );
}

async fn test_dynamic_filter_unallowed_filter_key(_ctx: &TestContext, client: &mut TestClient) {
    let url = "/v1/user/all?invalid_filter_parameter=123";
    let (status, _) = client.get(url).await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "Backend allowed filtering on unallowed query parameter"
    );
}

async fn test_dynamic_filter_invalid_date_format(_ctx: &TestContext, client: &mut TestClient) {
    let url = "/v1/user/all?createdAt_start=2024-invalid-format";
    let (status, _) = client.get(url).await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "Backend allowed invalid date format without returning 400"
    );
}

async fn test_dynamic_filter_date_range(_ctx: &TestContext, client: &mut TestClient) {
    let unique_suffix = uuid::Uuid::new_v4().to_string()[..8].to_string();
    let email = format!("date_test_{}@email.com", unique_suffix);
    let user_payload = json!({
        "name": format!("Date Test User {}", unique_suffix),
        "email": email,
        "password": "Password123!",
        "id_role": "administrator"
    });
    let (u_status, u_resp) = client.post_json("/v1/user", &user_payload).await;
    assert_eq!(u_status, StatusCode::CREATED);
    let user_id = read_body_json(u_resp).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();

    let url = format!(
        "/v1/user/all?page=0&size=25&createdAt_start={}&createdAt_end={}",
        today, today
    );
    let (status, resp) = client.get(&url).await;
    assert_eq!(status, StatusCode::OK);

    let data = read_body_json(resp).await;
    assert!(data.get("items").is_some());
    let items = data["items"].as_array().unwrap();

    let ids: Vec<String> = items
        .iter()
        .map(|u| u["id"].as_str().unwrap().to_string())
        .collect();
    assert!(
        ids.contains(&user_id),
        "Expected to find newly registered user in today's date range"
    );

    let (d_status, _) = client.delete(&format!("/v1/user/{}", user_id)).await;
    assert_eq!(d_status, StatusCode::NO_CONTENT);
}

async fn test_dynamic_filter_active_status(_ctx: &TestContext, client: &mut TestClient) {
    let unique_suffix = uuid::Uuid::new_v4().to_string()[..8].to_string();

    let role_payload = json!({
        "name": format!("Filter Test Role {}", unique_suffix),
        "description": "Role for filtering tests",
        "permissions": []
    });
    let (r_status, r_resp) = client.post_json("/v1/role", &role_payload).await;
    assert_eq!(r_status, StatusCode::CREATED);
    let role_id = read_body_json(r_resp).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    let email = format!("filter_test_{}@email.com", unique_suffix);
    let user_payload = json!({
        "name": format!("Filter Test User {}", unique_suffix),
        "email": email,
        "password": "Password123!",
        "id_role": role_id
    });
    let (u_status, u_resp) = client.post_json("/v1/user", &user_payload).await;
    assert_eq!(u_status, StatusCode::CREATED);
    let user_id = read_body_json(u_resp).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    let (_, all_resp) = client.get("/v1/user/all?page=0&size=100").await;
    let all_items = read_body_json(all_resp).await["items"]
        .as_array()
        .unwrap()
        .clone();
    let all_ids: Vec<String> = all_items
        .iter()
        .map(|u| u["id"].as_str().unwrap().to_string())
        .collect();
    assert!(
        all_ids.contains(&user_id),
        "User should be visible on /all by default"
    );

    let (_, root_resp) = client.get("/v1/user?page=0&size=100").await;
    let root_items = read_body_json(root_resp).await["items"]
        .as_array()
        .unwrap()
        .clone();
    let root_ids: Vec<String> = root_items
        .iter()
        .map(|u| u["id"].as_str().unwrap().to_string())
        .collect();
    assert!(
        root_ids.contains(&user_id),
        "User should be visible on root /user by default"
    );

    let deact_payload = json!({ "active": false });
    let (deact_status, deact_resp) = client
        .patch_json(&format!("/v1/user/{}/status", user_id), &deact_payload)
        .await;
    assert_eq!(deact_status, StatusCode::OK);
    assert!(!read_body_json(deact_resp).await["active"]
        .as_bool()
        .unwrap());

    let (_, root_no_param_resp) = client.get("/v1/user?page=0&size=100").await;
    let root_no_param_items = read_body_json(root_no_param_resp).await["items"]
        .as_array()
        .unwrap()
        .clone();
    let root_no_param_ids: Vec<String> = root_no_param_items
        .iter()
        .map(|u| u["id"].as_str().unwrap().to_string())
        .collect();
    assert!(
        !root_no_param_ids.contains(&user_id),
        "Deactivated user should NOT be visible on root without explicit parameter"
    );

    let (_, all_no_param_resp) = client.get("/v1/user/all?page=0&size=100").await;
    let all_no_param_items = read_body_json(all_no_param_resp).await["items"]
        .as_array()
        .unwrap()
        .clone();
    let all_no_param_ids: Vec<String> = all_no_param_items
        .iter()
        .map(|u| u["id"].as_str().unwrap().to_string())
        .collect();
    assert!(
        all_no_param_ids.contains(&user_id),
        "Deactivated user SHOULD be visible on /all without explicit parameter"
    );

    let (_, active_resp) = client.get("/v1/user/all?page=0&size=100&active=true").await;
    let active_users = read_body_json(active_resp).await["items"]
        .as_array()
        .unwrap()
        .clone();
    for u in &active_users {
        assert!(
            u["active"].as_bool().unwrap(),
            "Returned user active status was False when active=true requested"
        );
    }
    let active_ids: Vec<String> = active_users
        .iter()
        .map(|u| u["id"].as_str().unwrap().to_string())
        .collect();
    assert!(
        !active_ids.contains(&user_id),
        "Deactivated user should not be visible when active=true requested"
    );

    let (_, inactive_resp) = client
        .get("/v1/user/all?page=0&size=100&active=false")
        .await;
    let inactive_users = read_body_json(inactive_resp).await["items"]
        .as_array()
        .unwrap()
        .clone();
    for u in &inactive_users {
        assert!(
            !u["active"].as_bool().unwrap(),
            "Returned user active status was True when active=false requested"
        );
    }
    let inactive_ids: Vec<String> = inactive_users
        .iter()
        .map(|u| u["id"].as_str().unwrap().to_string())
        .collect();
    assert!(
        inactive_ids.contains(&user_id),
        "Deactivated user should be visible when active=false requested"
    );

    let (d_status, _) = client.delete(&format!("/v1/user/{}", user_id)).await;
    assert_eq!(d_status, StatusCode::NO_CONTENT);
    let (dr_status, _) = client.delete(&format!("/v1/role/{}", role_id)).await;
    assert_eq!(dr_status, StatusCode::NO_CONTENT);
}

async fn test_pagination_size_limit(_ctx: &TestContext, client: &mut TestClient) {
    let (s1, _) = client.get("/v1/user/all?page=0&size=100").await;
    assert_eq!(s1, StatusCode::OK);

    let (s2, _) = client.get("/v1/user/all?page=0&size=101").await;
    assert_eq!(s2, StatusCode::BAD_REQUEST);

    let (s3, _) = client.get("/v1/user?page=0&size=100").await;
    assert_eq!(s3, StatusCode::OK);

    let (s4, _) = client.get("/v1/user?page=0&size=101").await;
    assert_eq!(s4, StatusCode::BAD_REQUEST);
}

async fn test_listing_sorting(_ctx: &TestContext, client: &mut TestClient) {
    let unique_suffix = uuid::Uuid::new_v4().to_string()[..8].to_string();
    let user_payload = json!({
        "name": format!("AAA Filter User {}", unique_suffix),
        "email": format!("aaa_{}@email.com", unique_suffix),
        "password": "Password123!",
        "id_role": "administrator"
    });
    let (c_status, u_resp) = client.post_json("/v1/user", &user_payload).await;
    assert_eq!(c_status, StatusCode::CREATED);
    let user_id = read_body_json(u_resp).await["id"]
        .as_str()
        .unwrap()
        .to_string();

    let (status_asc, resp_asc) = client
        .get("/v1/user/all?page=0&size=100&orderBy=name&orderDirection=asc")
        .await;
    assert_eq!(status_asc, StatusCode::OK);
    let items_asc = read_body_json(resp_asc).await["items"]
        .as_array()
        .unwrap()
        .clone();
    let names_asc: Vec<String> = items_asc
        .iter()
        .map(|u| u["name"].as_str().unwrap().to_lowercase())
        .collect();
    let mut sorted_asc = names_asc.clone();
    sorted_asc.sort();
    assert_eq!(
        names_asc, sorted_asc,
        "Names are not sorted ascending: {:?}",
        names_asc
    );

    let (status_desc, resp_desc) = client
        .get("/v1/user/all?page=0&size=100&orderBy=name&orderDirection=desc")
        .await;
    assert_eq!(status_desc, StatusCode::OK);
    let items_desc = read_body_json(resp_desc).await["items"]
        .as_array()
        .unwrap()
        .clone();
    let names_desc: Vec<String> = items_desc
        .iter()
        .map(|u| u["name"].as_str().unwrap().to_lowercase())
        .collect();
    let mut sorted_desc = names_desc.clone();
    sorted_desc.sort_by(|a, b| b.cmp(a));
    assert_eq!(
        names_desc, sorted_desc,
        "Names are not sorted descending: {:?}",
        names_desc
    );

    let (status_bad, _) = client
        .get("/v1/user/all?page=0&size=10&orderBy=invalid_column_name")
        .await;
    assert_eq!(status_bad, StatusCode::BAD_REQUEST);

    let (d_status, _) = client.delete(&format!("/v1/user/{}", user_id)).await;
    assert_eq!(d_status, StatusCode::NO_CONTENT);
}
