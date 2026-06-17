use axum::Router;
use backend_rust::{
    config::AppConfig,
    infra::{
        bootstrap::bootstrap_database, cache::Cache, database, messaging::MessagingProvider,
        storage::StorageProvider,
    },
    middleware,
    migration::Migrator,
    modules,
};
use sea_orm_migration::MigratorTrait;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("falhou ao instalar handler de Ctrl+C");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("falhou ao instalar handler de SIGTERM")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("Sinal de desligamento recebido. Encerrando servidor graciosamente...");
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    tracing::info!("🚀 Iniciando Mage Backend Boilerplate (Rust)...");

    let config = AppConfig::load();

    let db = {
        let mut retries = 5;
        loop {
            match database::connect(&config.database_url).await {
                Ok(conn) => break conn,
                Err(e) if retries > 0 => {
                    tracing::warn!(
                        "Falha ao conectar com PostgreSQL: {}. Tentativas restantes: {}",
                        e,
                        retries
                    );
                    retries -= 1;
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
                Err(e) => panic!(
                    "Falha fatal ao conectar com banco de dados PostgreSQL: {}",
                    e
                ),
            }
        }
    };

    {
        let mut retries = 5;
        loop {
            match Migrator::up(&db, None).await {
                Ok(_) => break,
                Err(e) if retries > 0 => {
                    tracing::warn!(
                        "Falha ao executar migrações: {}. Tentativas restantes: {}",
                        e,
                        retries
                    );
                    retries -= 1;
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
                Err(e) => panic!("Falha fatal ao executar migrações do banco de dados: {}", e),
            }
        }
    }
    tracing::info!("✅ Migrações aplicadas com sucesso!");

    {
        let mut retries = 5;
        loop {
            match bootstrap_database(&db).await {
                Ok(_) => break,
                Err(e) if retries > 0 => {
                    tracing::warn!(
                        "Falha ao executar bootstrap: {}. Tentativas restantes: {}",
                        e,
                        retries
                    );
                    retries -= 1;
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
                Err(e) => panic!("Falha fatal ao executar bootstrap do banco de dados: {}", e),
            }
        }
    }

    let cache = Cache::new(&config.redis_url);
    {
        let mut retries = 5;
        loop {
            match cache.pool.get().await {
                Ok(mut conn) => {
                    let _: Result<(), _> = redis::cmd("PING").query_async(&mut conn).await;
                    break;
                }
                Err(e) if retries > 0 => {
                    tracing::warn!(
                        "Falha ao conectar com Redis: {}. Tentativas restantes: {}",
                        e,
                        retries
                    );
                    retries -= 1;
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
                Err(e) => panic!("Falha fatal ao conectar com o Redis: {}", e),
            }
        }
    }
    tracing::info!("✅ Conexão com Redis Cache estabelecida.");

    {
        let mut retries = 5;
        loop {
            match MessagingProvider::init(&config).await {
                Ok(_) => break,
                Err(e) if retries > 0 => {
                    tracing::warn!(
                        "Falha ao conectar com RabbitMQ: {}. Tentativas restantes: {}",
                        e,
                        retries
                    );
                    retries -= 1;
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
                Err(e) => panic!("Falha fatal ao inicializar provedor RabbitMQ: {}", e),
            }
        }
    }

    if config.messaging_enabled {
        tracing::info!("✅ Conexão com RabbitMQ estabelecida.");
    } else {
        tracing::info!("ℹ️ Integração com RabbitMQ desabilitada via configurações.");
    }

    {
        let mut retries = 5;
        loop {
            match StorageProvider::init(&config).await {
                Ok(_) => break,
                Err(e) if retries > 0 => {
                    tracing::warn!(
                        "Falha ao inicializar StorageProvider: {}. Tentativas restantes: {}",
                        e,
                        retries
                    );
                    retries -= 1;
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
                Err(e) => panic!("Falha fatal ao inicializar o provedor de storage: {}", e),
            }
        }
    }
    tracing::info!("✅ Conexão com Storage Provider estabelecida.");

    let api_router = modules::app_router(db.clone(), cache.clone(), config.clone());
    let obs_router = modules::observability::router(db.clone(), cache.clone());

    let cors_origins: Vec<axum::http::HeaderValue> = config
        .cors_allowed_origins
        .split(',')
        .filter(|s| !s.trim().is_empty())
        .filter_map(|s| s.trim().parse().ok())
        .collect();

    let cors = if config.environment == "development" && cors_origins.is_empty() {
        CorsLayer::new()
            .allow_origin(Any)
            .allow_headers(Any)
            .allow_methods(Any)
    } else if !cors_origins.is_empty() {
        CorsLayer::new()
            .allow_origin(cors_origins)
            .allow_headers(Any)
            .allow_methods(Any)
    } else {
        CorsLayer::new()
    };

    let app = Router::new()
        .merge(api_router)
        .merge(obs_router)
        .nest_service("/uploads", tower_http::services::ServeDir::new("uploads"))
        .layer(axum::middleware::from_fn_with_state(
            db.clone(),
            middleware::error_log::error_logging_middleware,
        ))
        .layer(axum::middleware::from_fn_with_state(
            db.clone(),
            middleware::audit::audit_middleware,
        ))
        .layer(axum::middleware::from_fn_with_state(
            cache.clone(),
            middleware::rate_limit::rate_limit_middleware,
        ))
        .layer(axum::middleware::from_fn(
            middleware::request_log::request_logging_middleware,
        ))
        .layer(axum::middleware::from_fn(
            modules::observability::track_metrics_middleware,
        ))
        .layer(cors);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!(
        "⚡ Servidor rodando com sucesso no endereço http://{}",
        addr
    );
    tracing::info!(
        "📖 Documentação Swagger disponível em http://{}/v1/docs",
        addr
    );

    let listener = TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();

    if config.messaging_enabled {
        if let Err(e) = MessagingProvider::get().disconnect().await {
            tracing::error!("Erro ao desconectar RabbitMQ graciosamente: {}", e);
        }
    }
}
