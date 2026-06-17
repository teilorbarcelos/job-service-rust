use crate::{
    config::AppConfig, infra::cache::Cache, middleware::auth::auth_middleware,
    modules::audit::controller::list_audit_logs_handler,
};
use axum::{middleware::from_fn_with_state, routing::get, Router};
use sea_orm::DatabaseConnection;

pub fn router(db: DatabaseConnection, cache: Cache, config: AppConfig) -> Router {
    let state = (db.clone(), cache.clone(), config.clone());

    let secure_routes = Router::new()
        .route("/all", get(list_audit_logs_handler))
        .layer(from_fn_with_state(
            (cache.clone(), config.clone()),
            auth_middleware,
        ))
        .with_state(state);

    Router::new().nest("/v1/audit", secure_routes)
}
