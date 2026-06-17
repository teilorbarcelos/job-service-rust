use crate::{
    auth_route,
    config::AppConfig,
    infra::cache::Cache,
    middleware::auth::auth_middleware,
    modules::role::controller::{
        create_role_handler, delete_role_handler, get_role_handler, list_features_handler,
        list_roles_handler, toggle_role_status_handler, update_role_handler,
    },
};
use axum::{
    handler::Handler,
    middleware::from_fn_with_state,
    routing::{delete, get, patch},
    Router,
};
use sea_orm::DatabaseConnection;

pub fn router(db: DatabaseConnection, cache: Cache, config: AppConfig) -> Router {
    let state = (db.clone(), cache.clone(), config.clone());

    let secure_routes = Router::new()
        .route(
            "/features",
            get(auth_route!(
                db,
                cache,
                "role",
                "view",
                list_features_handler
            )),
        )
        .route(
            "/all",
            get(auth_route!(db, cache, "role", "view", list_roles_handler)),
        )
        .route(
            "/",
            get(auth_route!(db, cache, "role", "view", list_roles_handler)).post(auth_route!(
                db,
                cache,
                "role",
                "create",
                create_role_handler
            )),
        )
        .route(
            "/:id",
            get(auth_route!(db, cache, "role", "view", get_role_handler)).put(auth_route!(
                db,
                cache,
                "role",
                "create",
                update_role_handler
            )),
        )
        .route(
            "/:id",
            delete(auth_route!(
                db,
                cache,
                "role",
                "delete",
                delete_role_handler
            )),
        )
        .route(
            "/:id/status",
            patch(auth_route!(
                db,
                cache,
                "role",
                "activate",
                toggle_role_status_handler
            )),
        )
        .layer(from_fn_with_state(
            (cache.clone(), config.clone()),
            auth_middleware,
        ))
        .with_state(state);

    Router::new().nest("/v1/role", secure_routes)
}
