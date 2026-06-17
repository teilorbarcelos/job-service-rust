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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_context_creation() {
        let signal = Arc::new(JobSignal::new());
        let ctx = JobContext { signal: signal.clone() };
        assert!(!ctx.signal.aborted());
    }

    #[test]
    fn test_job_result_new() {
        let result = JobResult::new("test".into(), JobStatus::Success, 100, None);
        assert_eq!(result.job, "test");
        assert_eq!(result.duration_ms, 100);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_job_result_with_error() {
        let result = JobResult::new("test".into(), JobStatus::Failed, 50, Some("err".into()));
        assert_eq!(result.status, JobStatus::Failed);
        assert_eq!(result.error, Some("err".into()));
    }

    #[test]
    fn test_job_result_clone() {
        let a = JobResult::new("a".into(), JobStatus::Success, 0, None);
        let b = a.clone();
        assert_eq!(a.job, b.job);
    }
}
