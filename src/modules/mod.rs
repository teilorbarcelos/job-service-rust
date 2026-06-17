pub mod audit;
pub mod audit_explorer;
pub mod auth;
pub mod dashboard;
pub mod observability;
pub mod product;
pub mod role;
pub mod upload;
pub mod user;

use crate::{config::AppConfig, infra::cache::Cache};
use axum::Router;
use sea_orm::DatabaseConnection;

pub fn app_router(db: DatabaseConnection, cache: Cache, config: AppConfig) -> Router {
    let mut router = Router::new()
        .merge(user::router(db.clone(), cache.clone(), config.clone()))
        .merge(role::router(db.clone(), cache.clone(), config.clone()))
        .merge(product::router(db.clone(), cache.clone(), config.clone()))
        .merge(audit::router(db.clone(), cache.clone(), config.clone()))
        .merge(audit_explorer::router(
            db.clone(),
            cache.clone(),
            config.clone(),
        ))
        .merge(dashboard::router(db.clone(), cache.clone(), config.clone()))
        .merge(upload::router(db.clone(), cache.clone(), config.clone()));

    if config.auth_mode == "local" {
        router = router.merge(auth::router(db, cache, config));
    } else {
        tracing::info!("ℹ️ AUTH_MODE=remote — rota /v1/auth/* desabilitada no monólito");
    }

    router
}
