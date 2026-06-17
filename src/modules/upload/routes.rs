use crate::{
    config::AppConfig, infra::cache::Cache, middleware::auth::auth_middleware,
    modules::upload::controller::upload_file_handler,
};
use axum::{middleware::from_fn_with_state, routing::post, Router};
use sea_orm::DatabaseConnection;

pub fn router(db: DatabaseConnection, cache: Cache, config: AppConfig) -> Router {
    let state = (db.clone(), cache.clone(), config.clone());

    let secure_routes = Router::new()
        .route("/", post(upload_file_handler))
        .layer(from_fn_with_state(
            (cache.clone(), config.clone()),
            auth_middleware,
        ))
        .with_state(state);

    Router::new().nest("/v1/upload", secure_routes)
}
