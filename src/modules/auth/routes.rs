use crate::{
    config::AppConfig,
    infra::cache::Cache,
    middleware::auth::auth_middleware,
    modules::auth::controller::{get_me_handler, login_handler, logout_handler, refresh_handler},
};
use axum::{
    middleware::from_fn_with_state,
    routing::{get, post},
    Router,
};
use sea_orm::DatabaseConnection;

pub fn router(db: DatabaseConnection, cache: Cache, config: AppConfig) -> Router {
    let state = (db.clone(), cache.clone(), config.clone());

    let public_routes = Router::new()
        .route("/login", post(login_handler))
        .route("/refresh", post(refresh_handler))
        .with_state(state.clone());

    let private_routes = Router::new()
        .route("/me", get(get_me_handler))
        .route("/logout", post(logout_handler))
        .layer(from_fn_with_state(
            (cache.clone(), config.clone()),
            auth_middleware,
        ))
        .with_state(state);

    Router::new().nest("/v1/auth", public_routes.merge(private_routes))
}
