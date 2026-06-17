use crate::common::{read_body_json, read_body_string, TestClient, TestContext};
use axum::http::StatusCode;
use serde_json::json;
use std::fs;
use std::path::Path;

pub async fn run(ctx: &TestContext) {
    println!("=== Running Upload and Local Storage Tests ===");
    ctx.clear_database().await;
    ctx.reset_rate_limiter().await;

    let mut client = TestClient::new(ctx.router.clone());

    let boundary = "------------------------1234567890";
    let body_str = format!(
        "--{boundary}\r\n\
         Content-Disposition: form-data; name=\"file\"; filename=\"test_upload.txt\"\r\n\
         Content-Type: text/plain\r\n\r\n\
         hello from test upload\r\n\
         --{boundary}--\r\n"
    );
    let content_type = format!("multipart/form-data; boundary={boundary}");

    let (anon_status, _) = client
        .request(
            "POST",
            "/v1/upload",
            axum::body::Body::from(body_str.clone()),
            Some(&content_type),
        )
        .await;
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

    let (status, resp) = client
        .request(
            "POST",
            "/v1/upload",
            axum::body::Body::from(body_str),
            Some(&content_type),
        )
        .await;
    assert_eq!(status, StatusCode::OK);

    let body = read_body_json(resp).await;
    let url = body["url"].as_str().expect("Response did not contain URL");
    assert!(url.starts_with("/uploads/"));
    assert!(url.contains("test_upload.txt"));

    let (get_status, get_resp) = client.get(url).await;
    assert_eq!(get_status, StatusCode::OK);
    let file_content = read_body_string(get_resp).await;
    assert_eq!(file_content, "hello from test upload");

    let path_on_disk = url.trim_start_matches('/');
    if Path::new(path_on_disk).exists() {
        let _ = fs::remove_file(path_on_disk);
    }

    let empty_body = format!(
        "--{boundary}\r\n\
         Content-Disposition: form-data; name=\"file\"; filename=\"empty.txt\"\r\n\
         Content-Type: text/plain\r\n\r\n\r\n\
         --{boundary}--\r\n"
    );
    let (empty_status, empty_resp) = client
        .request(
            "POST",
            "/v1/upload",
            axum::body::Body::from(empty_body),
            Some(&content_type),
        )
        .await;
    assert_eq!(empty_status, StatusCode::BAD_REQUEST);
    let empty_err = read_body_json(empty_resp).await;
    assert!(empty_err["message"]
        .as_str()
        .unwrap()
        .contains("File is empty"));

    let no_fields_body = format!("--{boundary}--\r\n");
    let (no_fields_status, _) = client
        .request(
            "POST",
            "/v1/upload",
            axum::body::Body::from(no_fields_body),
            Some(&content_type),
        )
        .await;
    assert_eq!(no_fields_status, StatusCode::BAD_REQUEST);
}
