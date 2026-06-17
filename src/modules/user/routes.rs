use crate::{
    auth_route,
    config::AppConfig,
    infra::cache::Cache,
    middleware::auth::auth_middleware,
    modules::user::controller::{
        create_user_handler, delete_user_handler, export_pdf_handler, get_user_handler,
        list_users_handler, toggle_user_status_handler, update_user_handler,
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
            "/export/pdf",
            get(auth_route!(db, cache, "user", "view", export_pdf_handler)),
        )
        .route(
            "/all",
            get(auth_route!(db, cache, "user", "view", list_users_handler)),
        )
        .route(
            "/",
            get(auth_route!(db, cache, "user", "view", list_users_handler)).post(auth_route!(
                db,
                cache,
                "user",
                "create",
                create_user_handler
            )),
        )
        .route(
            "/:id",
            get(auth_route!(db, cache, "user", "view", get_user_handler)).put(auth_route!(
                db,
                cache,
                "user",
                "create",
                update_user_handler
            )),
        )
        .route(
            "/:id",
            delete(auth_route!(
                db,
                cache,
                "user",
                "delete",
                delete_user_handler
            )),
        )
        .route(
            "/:id/status",
            patch(auth_route!(
                db,
                cache,
                "user",
                "activate",
                toggle_user_status_handler
            )),
        )
        .layer(from_fn_with_state(
            (cache.clone(), config.clone()),
            auth_middleware,
        ))
        .with_state(state);

    Router::new().nest("/v1/user", secure_routes)
}
