use crate::{errors::AppError, middleware::auth::CurrentUser};
use axum::{
    body::{to_bytes, Body},
    extract::State,
    http::{Method, Request},
    middleware::Next,
    response::Response,
};
use sea_orm::{ConnectionTrait, DatabaseConnection, Statement};
use serde_json::Value;
use uuid::Uuid;

fn scrub_json(val: &mut Value) {
    match val {
        Value::Object(map) => {
            for (k, v) in map.iter_mut() {
                let k_lower = k.to_lowercase();
                if k_lower.contains("password")
                    || k_lower.contains("token")
                    || k_lower.contains("secret")
                    || k_lower.contains("senha")
                {
                    *v = Value::String("[SCRUBBED]".to_string());
                } else {
                    scrub_json(v);
                }
            }
        }
        Value::Array(arr) => {
            for item in arr.iter_mut() {
                scrub_json(item);
            }
        }
        _ => {}
    }
}

fn scrub_body_str(raw_body: &str) -> String {
    if raw_body.trim().is_empty() {
        return "".to_string();
    }

    if let Ok(mut json_val) = serde_json::from_str::<Value>(raw_body) {
        scrub_json(&mut json_val);
        serde_json::to_string(&json_val).unwrap_or_else(|_| "".to_string())
    } else {
        raw_body.to_string()
    }
}

pub async fn audit_middleware(
    State(db): State<DatabaseConnection>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, AppError> {
    let method = req.method().clone();

    let is_mutation = method == Method::POST
        || method == Method::PUT
        || method == Method::DELETE
        || method == Method::PATCH;

    if !is_mutation {
        return Ok(next.run(req).await);
    }

    let (parts, body) = req.into_parts();

    let bytes = to_bytes(body, 1024 * 1024 * 10).await.unwrap_or_default();
    let raw_body = String::from_utf8_lossy(&bytes).into_owned();
    let scrubbed_body = scrub_body_str(&raw_body);

    let path_str = parts.uri.path().to_string();

    let reconstructed_req = Request::from_parts(parts, Body::from(bytes));

    let response = next.run(reconstructed_req).await;

    let current_user = response
        .extensions()
        .get::<CurrentUser>()
        .cloned()
        .or_else(|| response.extensions().get::<CurrentUser>().cloned());

    if method == Method::POST && path_str.contains("/v1/auth/login") && current_user.is_none() {
        return Ok(response);
    }

    let url_path = path_str.clone();

    let (class_name, table_name) = if path_str.contains("/v1/user") {
        ("User", "User")
    } else if path_str.contains("/v1/role") {
        ("Role", "Role")
    } else if path_str.contains("/v1/product") {
        ("Product", "Product")
    } else {
        ("Generic", "Generic")
    };

    let action_type = match method {
        Method::POST => "CREATE",
        Method::PUT => "UPDATE",
        Method::DELETE => "DELETE",
        _ => "MUTATE",
    };

    if let Some(user) = current_user {
        let audit_id = Uuid::new_v4().to_string();
        let user_id = user.id;
        let email = user.email;
        let execute_type = method.to_string();

        let query_str = r#"
            INSERT INTO audit.tb_audit (
                id, id_user, user_name, action_type, execute_type, class, function,
                params, raw, table_name, diff_value, original_url, method
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
        "#;

        let stmt = Statement::from_sql_and_values(
            db.get_database_backend(),
            query_str,
            vec![
                audit_id.into(),
                user_id.into(),
                email.into(),
                action_type.into(),
                execute_type.into(),
                class_name.into(),
                path_str.into(),
                "".into(),
                scrubbed_body.into(),
                table_name.into(),
                "{}".into(),
                url_path.into(),
                method.to_string().into(),
            ],
        );

        if let Err(err) = db.execute(stmt).await {
            tracing::error!("Falha ao escrever log de auditoria: {}", err);
        }
    }

    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scrub_body_str() {
        let input = r#"{"password": "secret123", "normal": "value"}"#;
        let output = scrub_body_str(input);
        assert!(output.contains("[SCRUBBED]"));
        assert!(output.contains("normal"));

        let invalid_input = "invalid json payload";
        let output_invalid = scrub_body_str(invalid_input);
        assert_eq!(output_invalid, "invalid json payload");
    }

    #[tokio::test]
    async fn test_audit_db_execute_failure() {
        dotenvy::dotenv().ok();
        let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            format!(
                "postgres://{}:{}@127.0.0.1:5432/backend_rust",
                "postgres", "postgres"
            )
        });
        if let Ok(db) = sea_orm::Database::connect(&database_url).await {
            let stmt = Statement::from_sql_and_values(
                db.get_database_backend(),
                "INSERT INTO audit.tb_audit (invalid_column) VALUES ($1)",
                vec![1.into()],
            );
            let _ = db.execute(stmt).await;
        }
    }
}
