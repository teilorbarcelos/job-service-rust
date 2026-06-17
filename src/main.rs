use std::sync::Arc;

use tokio::signal;
use tracing::info;

use job_service_rust::core::cron::CronExpressionAdapter;
use job_service_rust::infra::database::DatabasePool;
use job_service_rust::infra::health::DefaultHealthChecker;
use job_service_rust::infra::messaging::MessagingProvider;
use job_service_rust::infra::redis::RedisProvider;
use job_service_rust::jobs::register_jobs::register_jobs;
use job_service_rust::shared::config::load_config;
use job_service_rust::shared::logger::setup_tracing;

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Arc::new(load_config()?);
    setup_tracing(&config.log_level);

    info!("Starting job-service-rust");

    let db = Arc::new(DatabasePool::connect(&config.database).await?);

    let redis = Arc::new(RedisProvider::connect(&config.redis).await?);

    let rabbit = Arc::new(tokio::sync::Mutex::new(
        MessagingProvider::connect(&config.messaging).await?,
    ));

    let checker = Arc::new(DefaultHealthChecker {
        db: db.clone(),
        redis: redis.clone(),
        rabbit: rabbit.clone(),
    });

    let cron = Arc::new(CronExpressionAdapter);

    let scheduler = register_jobs(config.clone(), cron, checker)?;

    info!("Job scheduler started, waiting for shutdown signal");

    tokio::select! {
        _ = scheduler.run() => {},
        _ = shutdown_signal() => {
            info!("Shutdown signal received");
            scheduler.stop();
            scheduler.wait_for_running_jobs().await;

            let mut rabbit_guard = rabbit.lock().await;
            rabbit_guard.close().await;
            drop(rabbit_guard);

            db.close().await;
            info!("Shutdown complete");
        }
    }

    Ok(())
}
