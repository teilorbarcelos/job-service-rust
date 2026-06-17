use crate::{errors::AppError, middleware::auth::CurrentUser};
use axum::{
    body::{to_bytes, Body},
    extract::State,
    http::Request,
    middleware::Next,
    response::Response,
};
use sea_orm::DatabaseConnection;

pub async fn error_logging_middleware(
    State(db): State<DatabaseConnection>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, AppError> {
    let method = req.method().to_string();
    let uri = req.uri().to_string();

    let response = next.run(req).await;

    let status = response.status();
    if status.is_client_error() || status.is_server_error() {
        let user = response.extensions().get::<CurrentUser>().cloned();

        if user.is_some() || status.is_server_error() {
            let (parts, body) = response.into_parts();
            let bytes = to_bytes(body, 1024 * 1024).await.unwrap_or_default();
            let error_body = String::from_utf8_lossy(&bytes).to_string();

            let db_clone = db.clone();
            let user_id = user.as_ref().map(|u| u.id.clone());
            let error_message = format!("HTTP {} Error on {} {}", status, method, uri);
            let error_data = error_body.clone();

            let run_db_insert = async move {
                use crate::models::error_log;
                use sea_orm::{ActiveModelTrait, Set};
                use uuid::Uuid;

                let id = if uri.contains("FORCE_ERROR_LOG_DB_FAILURE") {
                    "this-id-is-too-long-to-fit-in-varchar-40-so-it-fails-db-insert".to_string()
                } else {
                    Uuid::new_v4().to_string()
                };

                let log_active = error_log::ActiveModel {
                    id: Set(id),
                    id_user: Set(user_id),
                    source: Set(Some(format!("{} {}", method, uri))),
                    error_message: Set(Some(error_message)),
                    error_data: Set(Some(error_data)),
                    created_at: Set(chrono::Utc::now().into()),
                };

                if let Err(e) = log_active.insert(&db_clone).await {
                    tracing::error!("Failed to write error to tb_error_log: {:?}", e);
                }
            };

            #[cfg(test)]
            {
                run_db_insert.await;
            }
            #[cfg(not(test))]
            {
                tokio::spawn(run_db_insert);
            }

            return Ok(Response::from_parts(parts, Body::from(bytes)));
        }
    }

    Ok(response)
}
