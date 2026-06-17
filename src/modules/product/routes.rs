use crate::{
    auth_route,
    config::AppConfig,
    infra::cache::Cache,
    middleware::auth::auth_middleware,
    modules::product::controller::{
        create_product_handler, delete_product_handler, get_product_handler, list_products_handler,
        toggle_product_status_handler, update_product_handler,
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
            "/",
            get(auth_route!(
                db,
                cache,
                "product",
                "view",
                list_products_handler
            ))
            .post(auth_route!(
                db,
                cache,
                "product",
                "create",
                create_product_handler
            )),
        )
        .route(
            "/all",
            get(auth_route!(
                db,
                cache,
                "product",
                "view",
                list_products_handler
            )),
        )
        .route(
            "/:id",
            get(auth_route!(
                db,
                cache,
                "product",
                "view",
                get_product_handler
            ))
            .put(auth_route!(
                db,
                cache,
                "product",
                "create",
                update_product_handler
            )),
        )
        .route(
            "/:id",
            delete(auth_route!(
                db,
                cache,
                "product",
                "delete",
                delete_product_handler
            )),
        )
        .route(
            "/:id/status",
            patch(auth_route!(
                db,
                cache,
                "product",
                "activate",
                toggle_product_status_handler
            )),
        )
        .layer(from_fn_with_state(
            (cache.clone(), config.clone()),
            auth_middleware,
        ))
        .with_state(state);

    Router::new().nest("/v1/product", secure_routes)
}
