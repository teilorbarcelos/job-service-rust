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
