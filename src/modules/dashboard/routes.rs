use crate::{
    auth_route, config::AppConfig, infra::cache::Cache, middleware::auth::auth_middleware,
    modules::dashboard::controller::get_stats_handler,
};
use axum::{handler::Handler, middleware::from_fn_with_state, routing::get, Router};
use sea_orm::DatabaseConnection;

pub fn router(db: DatabaseConnection, cache: Cache, config: AppConfig) -> Router {
    let state = (db.clone(), cache.clone(), config.clone());

    let secure_routes = Router::new()
        .route(
            "/stats",
            get(auth_route!(
                db,
                cache,
                "dashboard",
                "view",
                get_stats_handler
            )),
        )
        .layer(from_fn_with_state(
            (cache.clone(), config.clone()),
            auth_middleware,
        ))
        .with_state(state);

    Router::new().nest("/v1/dashboard", secure_routes)
}
