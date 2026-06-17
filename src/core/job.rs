use std::sync::Arc;

use super::errors::AppError;
use super::job_signal::JobSignal;
use super::job_status::JobStatus;
use crate::shared::config::AppConfig;

#[derive(Debug, Clone)]
pub struct JobContext {
    pub signal: Arc<JobSignal>,
}

#[derive(Debug, Clone)]
pub struct JobResult {
    pub job: String,
    pub status: JobStatus,
    pub duration_ms: u64,
    pub error: Option<String>,
}

#[async_trait::async_trait]
pub trait BaseJob: Send + Sync {
    fn name(&self) -> &str;
    fn schedule(&self) -> &str;
    fn description(&self) -> &str;
    fn enabled(&self) -> bool {
        true
    }
    async fn handle(&self, ctx: &JobContext, config: &AppConfig) -> Result<(), AppError>;
}

impl JobResult {
    pub fn new(job: String, status: JobStatus, duration_ms: u64, error: Option<String>) -> Self {
        Self {
            job,
            status,
            duration_ms,
            error,
        }
    }
}
