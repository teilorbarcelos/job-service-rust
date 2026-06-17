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
