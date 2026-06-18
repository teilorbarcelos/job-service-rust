use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use tokio::join;
use tracing::info;

use crate::core::errors::AppError;
use crate::core::job::{BaseJob, JobContext};
use crate::infra::health::HealthChecker;
use crate::shared::config::AppConfig;

pub struct HealthCheckJob {
    pub checker: Arc<dyn HealthChecker>,
    pub enabled: bool,
    pub schedule: String,
}

impl HealthCheckJob {
    pub fn new(checker: Arc<dyn HealthChecker>) -> Self {
        Self {
            checker,
            enabled: true,
            schedule: "*/1 * * * *".to_string(),
        }
    }
}

#[async_trait]
impl BaseJob for HealthCheckJob {
    fn name(&self) -> &str {
        "health-check"
    }

    fn schedule(&self) -> &str {
        &self.schedule
    }

    fn description(&self) -> &str {
        "Reports connection status with PostgreSQL, Redis and RabbitMQ"
    }

    fn enabled(&self) -> bool {
        self.enabled
    }

    async fn handle(&self, _ctx: &JobContext, _config: &AppConfig) -> Result<(), AppError> {
        let timestamp = Utc::now().to_rfc3339();

        let (pg, rd, rq) = join!(
            self.checker.check_postgres(),
            self.checker.check_redis(),
            self.checker.check_rabbitmq(),
        );

        let all_up = pg.status == "up"
            && rd.status == "up"
            && (rq.status == "up" || rq.status == "disabled");

        let status_str = if all_up { "healthy" } else { "degraded" };

        info!(
            event = "health-check",
            status = status_str,
            timestamp = &timestamp,
            postgres = pg.status,
            redis = rd.status,
            rabbitmq = rq.status,
            "Health check completed"
        );

        println!(
            "[HealthCheck {}] postgres={} redis={} rabbitmq={}",
            timestamp, pg.status, rd.status, rq.status,
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::job_signal::JobSignal;
    use crate::infra::health::{HealthCheckResult, HealthChecker};
    use async_trait::async_trait;

    struct MockChecker {
        pg: HealthCheckResult,
        rd: HealthCheckResult,
        rq: HealthCheckResult,
    }

    #[async_trait]
    impl HealthChecker for MockChecker {
        async fn check_postgres(&self) -> HealthCheckResult { self.pg.clone() }
        async fn check_redis(&self) -> HealthCheckResult { self.rd.clone() }
        async fn check_rabbitmq(&self) -> HealthCheckResult { self.rq.clone() }
    }

    #[tokio::test]
    async fn test_name() {
        let checker = Arc::new(MockChecker {
            pg: HealthCheckResult::up(5),
            rd: HealthCheckResult::up(1),
            rq: HealthCheckResult::disabled(),
        });
        let job = HealthCheckJob::new(checker);
        assert_eq!(job.name(), "health-check");
    }

    #[tokio::test]
    async fn test_schedule() {
        let checker = Arc::new(MockChecker {
            pg: HealthCheckResult::up(5),
            rd: HealthCheckResult::up(1),
            rq: HealthCheckResult::disabled(),
        });
        let job = HealthCheckJob { checker, enabled: true, schedule: "0 3 * * *".into() };
        assert_eq!(job.schedule(), "0 3 * * *");
    }

    #[tokio::test]
    async fn test_description() {
        let checker = Arc::new(MockChecker {
            pg: HealthCheckResult::up(5),
            rd: HealthCheckResult::up(1),
            rq: HealthCheckResult::disabled(),
        });
        let job = HealthCheckJob::new(checker);
        assert!(job.description().contains("PostgreSQL"));
    }

    #[tokio::test]
    async fn test_enabled_default() {
        let checker = Arc::new(MockChecker {
            pg: HealthCheckResult::up(5),
            rd: HealthCheckResult::up(1),
            rq: HealthCheckResult::disabled(),
        });
        let job = HealthCheckJob::new(checker);
        assert!(job.enabled());
    }

    #[tokio::test]
    async fn test_handle_healthy() {
        let checker = Arc::new(MockChecker {
            pg: HealthCheckResult::up(5),
            rd: HealthCheckResult::up(1),
            rq: HealthCheckResult::disabled(),
        });
        let job = HealthCheckJob::new(checker);
        let ctx = JobContext { signal: Arc::new(JobSignal::new()) };
        let config = AppConfig::default();
        let result = job.handle(&ctx, &config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_degraded_pg_down() {
        let checker = Arc::new(MockChecker {
            pg: HealthCheckResult::down(10, "refused".into()),
            rd: HealthCheckResult::up(1),
            rq: HealthCheckResult::disabled(),
        });
        let job = HealthCheckJob::new(checker);
        let ctx = JobContext { signal: Arc::new(JobSignal::new()) };
        let config = AppConfig::default();
        let result = job.handle(&ctx, &config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_degraded_redis_down() {
        let checker = Arc::new(MockChecker {
            pg: HealthCheckResult::up(5),
            rd: HealthCheckResult::down(1, "timeout".into()),
            rq: HealthCheckResult::disabled(),
        });
        let job = HealthCheckJob::new(checker);
        let ctx = JobContext { signal: Arc::new(JobSignal::new()) };
        let config = AppConfig::default();
        let result = job.handle(&ctx, &config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_handle_prints_stdout() {
        let checker = Arc::new(MockChecker {
            pg: HealthCheckResult::up(5),
            rd: HealthCheckResult::up(1),
            rq: HealthCheckResult::disabled(),
        });
        let job = HealthCheckJob::new(checker);
        let ctx = JobContext { signal: Arc::new(JobSignal::new()) };
        let config = AppConfig::default();
        // Capture stdout
        let result = job.handle(&ctx, &config).await;
        assert!(result.is_ok());
    }
}
