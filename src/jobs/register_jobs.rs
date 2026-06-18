use std::sync::Arc;

use crate::core::cron::CronAdapter;
use crate::core::scheduler::Scheduler;
use crate::infra::health::DefaultHealthChecker;
use crate::shared::config::AppConfig;

// [GENERATOR_IMPORTS]

pub fn register_jobs(
    config: Arc<AppConfig>,
    cron: Arc<dyn CronAdapter>,
    checker: Arc<DefaultHealthChecker>,
) -> Result<Scheduler, crate::core::errors::AppError> {
    let mut jobs: Vec<Arc<dyn crate::core::job::BaseJob>> = Vec::new();

    let mut hc = crate::jobs::health_check_job::HealthCheckJob::new(checker.clone());
    hc.enabled = config.jobs.health_check_enabled;
    hc.schedule = config.jobs.health_check_cron.clone();
    jobs.push(Arc::new(hc));

    // [GENERATOR_JOBS]

    Scheduler::new(jobs, cron, config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::cron::CronExpressionAdapter;
    use crate::infra::database::DatabasePool;
    use crate::infra::messaging::MessagingProvider;
    use crate::infra::redis::RedisProvider;
    use crate::shared::config::JobsConfig;

    async fn make_checker() -> Arc<DefaultHealthChecker> {
        let db_config = crate::shared::config::DatabaseConfig {
            driver: "sqlite".into(),
            url: "sqlite::memory:".into(),
        };
        let db = Arc::new(DatabasePool::connect(&db_config).await.unwrap());

        let redis_config = crate::shared::config::RedisConfig {
            host: "localhost".into(), port: 6379, password: "".into(), db: 0,
        };
        let redis = Arc::new(RedisProvider::connect(&redis_config).await.unwrap());

        let msg_config = crate::shared::config::MessagingConfig {
            enabled: false, host: "localhost".into(), port: 5672,
            user: "guest".into(), password: "guest".into(),
        };
        let rabbit = Arc::new(tokio::sync::Mutex::new(
            MessagingProvider::connect(&msg_config).await.unwrap(),
        ));

        Arc::new(DefaultHealthChecker { db, redis, rabbit })
    }

    #[tokio::test]
    async fn test_register_jobs_returns_scheduler() {
        let config = Arc::new(AppConfig::default());
        let cron = Arc::new(CronExpressionAdapter);
        let checker = make_checker().await;

        let result = register_jobs(config, cron, checker);
        assert!(result.is_ok());
        let scheduler = result.unwrap();
        let jobs = scheduler.list_jobs();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].name, "health-check");
    }

    #[tokio::test]
    async fn test_register_jobs_disabled() {
        let config = Arc::new(AppConfig {
            jobs: JobsConfig {
                health_check_enabled: false,
                ..AppConfig::default().jobs
            },
            ..AppConfig::default()
        });
        let cron = Arc::new(CronExpressionAdapter);
        let checker = make_checker().await;

        let result = register_jobs(config, cron, checker);
        assert!(result.is_ok());
        let scheduler = result.unwrap();
        let jobs = scheduler.list_jobs();
        assert_eq!(jobs.len(), 1);
        assert!(!jobs[0].enabled);
    }
}
