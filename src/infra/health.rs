use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;

use crate::infra::database::DatabasePool;
use crate::infra::messaging::MessagingProvider;
use crate::infra::redis::RedisProvider;

#[derive(Debug, Clone)]
pub struct HealthCheckResult {
    pub status: String,
    pub latency_ms: Option<u64>,
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_check_result_up() {
        let r = HealthCheckResult::up(5);
        assert_eq!(r.status, "up");
        assert_eq!(r.latency_ms, Some(5));
        assert!(r.error.is_none());
    }

    #[test]
    fn test_health_check_result_down() {
        let r = HealthCheckResult::down(10, "error msg".into());
        assert_eq!(r.status, "down");
        assert_eq!(r.latency_ms, Some(10));
        assert_eq!(r.error, Some("error msg".into()));
    }

    #[test]
    fn test_health_check_result_disabled() {
        let r = HealthCheckResult::disabled();
        assert_eq!(r.status, "disabled");
        assert!(r.latency_ms.is_none());
        assert!(r.error.is_none());
    }

    #[test]
    fn test_clone() {
        let a = HealthCheckResult::up(1);
        let b = a.clone();
        assert_eq!(a.status, b.status);
    }
}

impl HealthCheckResult {
    pub fn up(latency_ms: u64) -> Self {
        Self {
            status: "up".to_string(),
            latency_ms: Some(latency_ms),
            error: None,
        }
    }

    pub fn down(latency_ms: u64, error: String) -> Self {
        Self {
            status: "down".to_string(),
            latency_ms: Some(latency_ms),
            error: Some(error),
        }
    }

    pub fn disabled() -> Self {
        Self {
            status: "disabled".to_string(),
            latency_ms: None,
            error: None,
        }
    }
}

#[async_trait]
pub trait HealthChecker: Send + Sync {
    async fn check_postgres(&self) -> HealthCheckResult;
    async fn check_redis(&self) -> HealthCheckResult;
    async fn check_rabbitmq(&self) -> HealthCheckResult;
}

pub struct DefaultHealthChecker {
    pub db: Arc<DatabasePool>,
    pub redis: Arc<RedisProvider>,
    pub rabbit: Arc<tokio::sync::Mutex<MessagingProvider>>,
}

#[async_trait]
impl HealthChecker for DefaultHealthChecker {
    async fn check_postgres(&self) -> HealthCheckResult {
        let start = Instant::now();
        match self.db.ping().await {
            true => HealthCheckResult::up(start.elapsed().as_millis() as u64),
            false => HealthCheckResult::down(
                start.elapsed().as_millis() as u64,
                "Database ping failed".to_string(),
            ),
        }
    }

    async fn check_redis(&self) -> HealthCheckResult {
        let start = Instant::now();
        match self.redis.ping().await {
            Ok(()) => HealthCheckResult::up(start.elapsed().as_millis() as u64),
            Err(e) => HealthCheckResult::down(start.elapsed().as_millis() as u64, e.to_string()),
        }
    }

    async fn check_rabbitmq(&self) -> HealthCheckResult {
        let start = Instant::now();
        let rabbit = self.rabbit.lock().await;
        if rabbit.is_open() {
            HealthCheckResult::up(start.elapsed().as_millis() as u64)
        } else if !rabbit.enabled {
            HealthCheckResult::disabled()
        } else {
            HealthCheckResult::down(
                start.elapsed().as_millis() as u64,
                "Not connected".to_string(),
            )
        }
    }
}
