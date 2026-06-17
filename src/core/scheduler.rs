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

    pub fn is_running(&self, name: &str) -> bool {
        self.running.blocking_lock().contains(name)
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
                if self.running.blocking_lock().contains(&name) {
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
