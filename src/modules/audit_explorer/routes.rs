use crate::{
    config::AppConfig,
    infra::cache::Cache,
    middleware::auth::auth_middleware,
    modules::audit_explorer::controller::{
        get_audit_logs_handler, get_error_logs_handler, logs_html_handler,
    },
};
use axum::{middleware::from_fn_with_state, routing::get, Router};
use sea_orm::DatabaseConnection;

pub fn router(db: DatabaseConnection, cache: Cache, config: AppConfig) -> Router {
    let state = (db.clone(), cache.clone(), config.clone());

    let public_routes = Router::new().route("/admin/logs", get(logs_html_handler));

    let secure_routes = Router::new()
        .route("/admin/api/audit", get(get_audit_logs_handler))
        .route("/admin/api/errors", get(get_error_logs_handler))
        .layer(from_fn_with_state(
            (cache.clone(), config.clone()),
            auth_middleware,
        ))
        .with_state(state);

    public_routes.merge(secure_routes)
}
