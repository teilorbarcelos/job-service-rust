use crate::{
    auth_route,
    config::AppConfig,
    infra::cache::Cache,
    middleware::auth::auth_middleware,
    modules::{{entity_slug}}::controller::{
        create_{{entity_slug}}_handler, delete_{{entity_slug}}_handler, get_{{entity_slug}}_handler, list_{{entity_slug}}s_handler,
        toggle_{{entity_slug}}_status_handler, update_{{entity_slug}}_handler,
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
                "{{entity_slug}}",
                "view",
                list_{{entity_slug}}s_handler
            ))
            .post(auth_route!(
                db,
                cache,
                "{{entity_slug}}",
                "create",
                create_{{entity_slug}}_handler
            )),
        )
        .route(
            "/all",
            get(auth_route!(
                db,
                cache,
                "{{entity_slug}}",
                "view",
                list_{{entity_slug}}s_handler
            )),
        )
        .route(
            "/:id",
            get(auth_route!(
                db,
                cache,
                "{{entity_slug}}",
                "view",
                get_{{entity_slug}}_handler
            ))
            .put(auth_route!(
                db,
                cache,
                "{{entity_slug}}",
                "create",
                update_{{entity_slug}}_handler
            )),
        )
        .route(
            "/:id",
            delete(auth_route!(
                db,
                cache,
                "{{entity_slug}}",
                "delete",
                delete_{{entity_slug}}_handler
            )),
        )
        .route(
            "/:id/status",
            patch(auth_route!(
                db,
                cache,
                "{{entity_slug}}",
                "activate",
                toggle_{{entity_slug}}_status_handler
            )),
        )
        .layer(from_fn_with_state(
            (cache.clone(), config.clone()),
            auth_middleware,
        ))
        .with_state(state);

    Router::new().nest("/v1/{{entity_slug}}", secure_routes)
}
