use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::Utc;
use tokio::sync::Mutex;
use tokio::time::sleep;
use tracing::{info, warn};

use super::cron::CronAdapter;
use super::errors::AppError;
use super::job::BaseJob;
use super::job_info::JobInfo;
use super::job_signal::JobSignal;
use super::job::JobContext;
use crate::shared::config::AppConfig;

pub struct Scheduler {
    jobs: Vec<Arc<dyn BaseJob>>,
    next_runs: Mutex<Vec<Option<chrono::DateTime<chrono::Utc>>>>,
    running: Mutex<HashSet<String>>,
    cron: Arc<dyn CronAdapter>,
    stopped: AtomicBool,
    config: Arc<AppConfig>,
}

impl Scheduler {
    pub fn new(
        jobs: Vec<Arc<dyn BaseJob>>,
        cron: Arc<dyn CronAdapter>,
        config: Arc<AppConfig>,
    ) -> Result<Self, AppError> {
        let mut names = HashSet::new();
        for job in &jobs {
            if !names.insert(job.name().to_string()) {
                return Err(AppError::Validation(format!(
                    "Duplicate job name: {}",
                    job.name()
                )));
            }
        }
        Ok(Self {
            next_runs: Mutex::new(vec![None; jobs.len()]),
            running: Mutex::new(HashSet::new()),
            jobs,
            cron,
            stopped: AtomicBool::new(false),
            config,
        })
    }

    pub fn list_jobs(&self) -> Vec<JobInfo> {
        self.jobs
            .iter()
            .map(|j| {
                JobInfo::new(
                    j.name().to_string(),
                    j.schedule().to_string(),
                    j.enabled(),
                    j.description().to_string(),
                )
            })
            .collect()
    }

    pub async fn start(&self) -> Result<(), AppError> {
        for (i, job) in self.jobs.iter().enumerate() {
            if !job.enabled() {
                info!("Job disabled, will not be scheduled: {}", job.name());
                continue;
            }
            if !self.cron.is_valid(job.schedule()) {
                return Err(AppError::Validation(format!(
                    "Invalid cron expression for job {}: {}",
                    job.name(),
                    job.schedule()
                )));
            }
            let next = self.cron.next_run_date(job.schedule(), Utc::now());
            self.next_runs.lock().await[i] = next;
            info!(
                "Job scheduled: {} | cron: {} | desc: {}",
                job.name(),
                job.schedule(),
                job.description()
            );
        }
        Ok(())
    }

    pub fn stop(&self) {
        self.stopped.store(true, Ordering::SeqCst);
    }

    pub async fn wait_for_running_jobs(&self) {
        loop {
            let running = self.running.lock().await.len();
            if running == 0 {
                break;
            }
            sleep(Duration::from_millis(50)).await;
        }
    }

    pub async fn is_running(&self, name: &str) -> bool {
        self.running.lock().await.contains(name)
    }

    pub async fn tick(&self) {
        let now = Utc::now();
        let mut next_runs = self.next_runs.lock().await;

        for (i, job) in self.jobs.iter().enumerate() {
            if !job.enabled() {
                continue;
            }

            let should_run = match next_runs[i] {
                None => true,
                Some(next) => now >= next,
            };

            if should_run {
                let name = job.name().to_string();
                if self.running.lock().await.contains(&name) {
                    warn!("Job {} still running, skipping", name);
                    next_runs[i] = self.cron.next_run_date(job.schedule(), Utc::now());
                    continue;
                }

                self.running.lock().await.insert(name.clone());
                let signal = Arc::new(JobSignal::new());
                let ctx = JobContext { signal };

                let start = Instant::now();
                let result = tokio::time::timeout(
                    Duration::from_millis(self.config.job_execution_timeout_ms),
                    job.handle(&ctx, &self.config),
                )
                .await;

                let duration_ms = start.elapsed().as_millis() as u64;

                match result {
                    Ok(Ok(())) => {
                        info!("Job {} finished successfully in {}ms", name, duration_ms);
                    }
                    Ok(Err(e)) => {
                        warn!("Job {} failed: {} ({}ms)", name, e, duration_ms);
                    }
                    Err(_) => {
                        warn!("Job {} timed out after {}ms", name, duration_ms);
                    }
                }

                self.running.lock().await.remove(&name);
                next_runs[i] = self.cron.next_run_date(job.schedule(), Utc::now());
            }
        }
    }

    pub async fn run(&self) {
        self.start().await.ok();

        loop {
            if self.stopped.load(Ordering::SeqCst) {
                break;
            }
            self.tick().await;
            sleep(Duration::from_secs(1)).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::DateTime;
    use crate::core::job::BaseJob;
    use async_trait::async_trait;
    use std::sync::Arc;

    struct TestJob {
        name: String,
        schedule: String,
        description: String,
        enabled: bool,
    }

    #[async_trait]
    impl BaseJob for TestJob {
        fn name(&self) -> &str { &self.name }
        fn schedule(&self) -> &str { &self.schedule }
        fn description(&self) -> &str { &self.description }
        fn enabled(&self) -> bool { self.enabled }
        async fn handle(&self, _ctx: &JobContext, _config: &AppConfig) -> Result<(), AppError> {
            Ok(())
        }
    }

    fn make_test_job(name: &str, schedule: &str) -> Arc<dyn BaseJob> {
        Arc::new(TestJob {
            name: name.to_string(),
            schedule: schedule.to_string(),
            description: "test".to_string(),
            enabled: true,
        })
    }

    fn make_disabled_job(name: &str, schedule: &str) -> Arc<dyn BaseJob> {
        Arc::new(TestJob {
            name: name.to_string(),
            schedule: schedule.to_string(),
            description: "disabled".to_string(),
            enabled: false,
        })
    }

    fn make_mock_cron() -> Arc<dyn CronAdapter> {
        Arc::new(MockCronAdapter)
    }

    struct MockCronAdapter;
    impl CronAdapter for MockCronAdapter {
        fn is_valid(&self, expr: &str) -> bool {
            expr != "invalid"
        }
        fn next_run_date(&self, _expr: &str, _from: DateTime<Utc>) -> Option<DateTime<Utc>> {
            Some(Utc::now() + chrono::Duration::minutes(1))
        }
    }

    #[tokio::test]
    async fn test_list_jobs() {
        let config = Arc::new(AppConfig::default());
        let jobs = vec![make_test_job("a", "*/5 * * * * *"), make_test_job("b", "*/10 * * * * *")];
        let s = Scheduler::new(jobs, make_mock_cron(), config).unwrap();
        let list = s.list_jobs();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].name, "a");
        assert_eq!(list[1].name, "b");
    }

    #[tokio::test]
    async fn test_duplicate_name_errors() {
        let config = Arc::new(AppConfig::default());
        let jobs = vec![make_test_job("dup", "*/5 * * * * *"), make_test_job("dup", "*/5 * * * * *")];
        let result = Scheduler::new(jobs, make_mock_cron(), config);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_start_validates_cron() {
        let config = Arc::new(AppConfig::default());
        let invalid_job = Arc::new(TestJob {
            name: "bad".to_string(),
            schedule: "invalid".to_string(),
            description: "".to_string(),
            enabled: true,
        });
        let s = Scheduler::new(vec![invalid_job], make_mock_cron(), config).unwrap();
        let result = s.start().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_start_disabled_job() {
        let config = Arc::new(AppConfig::default());
        let jobs = vec![make_disabled_job("a", "*/5 * * * * *")];
        let s = Scheduler::new(jobs, make_mock_cron(), config).unwrap();
        let result = s.start().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_is_running() {
        let config = Arc::new(AppConfig::default());
        let s = Scheduler::new(vec![], make_mock_cron(), config).unwrap();
        assert!(!s.is_running("nonexistent").await);
    }

    #[tokio::test]
    async fn test_stop_and_wait() {
        let config = Arc::new(AppConfig::default());
        let s = Scheduler::new(vec![], make_mock_cron(), config).unwrap();
        s.stop();
        s.wait_for_running_jobs().await;
    }

    #[tokio::test]
    async fn test_start_ok() {
        let config = Arc::new(AppConfig::default());
        let jobs = vec![make_test_job("ok", "*/5 * * * * *")];
        let s = Scheduler::new(jobs, make_mock_cron(), config).unwrap();
        let result = s.start().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_tick_with_no_jobs() {
        let config = Arc::new(AppConfig::default());
        let s = Scheduler::new(vec![], make_mock_cron(), config).unwrap();
        s.start().await.unwrap();
        s.tick().await;
    }

    #[tokio::test]
    async fn test_tick_executes_job() {
        let executed = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let executed_clone = executed.clone();

        struct ExecJob {
            executed: Arc<std::sync::atomic::AtomicBool>,
        }
        #[async_trait]
        impl BaseJob for ExecJob {
            fn name(&self) -> &str { "exec-me" }
            fn schedule(&self) -> &str { "*/1 * * * * *" }
            fn description(&self) -> &str { "" }
            fn enabled(&self) -> bool { true }
            async fn handle(&self, _ctx: &JobContext, _config: &AppConfig) -> Result<(), AppError> {
                self.executed.store(true, std::sync::atomic::Ordering::SeqCst);
                Ok(())
            }
        }

        let job = Arc::new(ExecJob { executed: executed_clone });
        let jobs: Vec<Arc<dyn BaseJob>> = vec![job];
        let cron = make_mock_cron();
        let config = Arc::new(AppConfig::default());
        let s = Scheduler::new(jobs, cron, config).unwrap();
        s.start().await.unwrap();

        // Overwrite next run to be in the past
        s.next_runs.lock().await[0] = Some(Utc::now() - chrono::Duration::hours(1));

        s.tick().await;

        assert!(executed.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_tick_skips_disabled_job() {
        let config = Arc::new(AppConfig::default());
        let jobs = vec![make_disabled_job("disabled", "*/1 * * * * *")];
        let s = Scheduler::new(jobs, make_mock_cron(), config).unwrap();
        s.start().await.unwrap();
        s.tick().await;
    }

    #[tokio::test]
    async fn test_tick_with_null_next_run() {
        struct NullCron;
        impl CronAdapter for NullCron {
            fn is_valid(&self, _expr: &str) -> bool { true }
            fn next_run_date(&self, _expr: &str, _from: DateTime<Utc>) -> Option<DateTime<Utc>> { None }
        }

        struct NullJob;
        #[async_trait]
        impl BaseJob for NullJob {
            fn name(&self) -> &str { "null" }
            fn schedule(&self) -> &str { "*/1 * * * * *" }
            fn description(&self) -> &str { "" }
            fn enabled(&self) -> bool { true }
            async fn handle(&self, _ctx: &JobContext, _config: &AppConfig) -> Result<(), AppError> { Ok(()) }
        }

        let config = Arc::new(AppConfig::default());
        let s = Scheduler::new(
            vec![Arc::new(NullJob)],
            Arc::new(NullCron),
            config,
        ).unwrap();
        s.start().await.unwrap();
        s.tick().await;
    }

    #[tokio::test]
    async fn test_tick_handle_error() {
        struct ErrJob;
        #[async_trait]
        impl BaseJob for ErrJob {
            fn name(&self) -> &str { "err" }
            fn schedule(&self) -> &str { "*/1 * * * * *" }
            fn description(&self) -> &str { "" }
            fn enabled(&self) -> bool { true }
            async fn handle(&self, _ctx: &JobContext, _config: &AppConfig) -> Result<(), AppError> {
                Err(AppError::Job("failed".into()))
            }
        }

        struct ImmediateCron;
        impl CronAdapter for ImmediateCron {
            fn is_valid(&self, _expr: &str) -> bool { true }
            fn next_run_date(&self, _expr: &str, _from: DateTime<Utc>) -> Option<DateTime<Utc>> {
                Some(Utc::now() - chrono::Duration::hours(1))
            }
        }

        let config = Arc::new(AppConfig::default());
        let s = Scheduler::new(
            vec![Arc::new(ErrJob)],
            Arc::new(ImmediateCron),
            config,
        ).unwrap();
        s.start().await.unwrap();
        s.tick().await;
    }
}
