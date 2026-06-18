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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::database::DatabasePool;
    use crate::infra::messaging::MessagingProvider;
    use crate::infra::redis::RedisProvider;
    use crate::shared::config::{DatabaseConfig, MessagingConfig, RedisConfig};
    use std::sync::Arc;

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

    async fn make_checker() -> DefaultHealthChecker {
        let db = Arc::new(
            DatabasePool::connect(&DatabaseConfig {
                driver: "sqlite".into(),
                url: "sqlite::memory:".into(),
            })
            .await
            .unwrap(),
        );
        let redis = Arc::new(
            RedisProvider::connect(&RedisConfig {
                host: "localhost".into(),
                port: 6379,
                password: "".into(),
                db: 0,
            })
            .await
            .unwrap(),
        );
        let rabbit = Arc::new(tokio::sync::Mutex::new(
            MessagingProvider::connect(&MessagingConfig {
                enabled: false,
                host: "localhost".into(),
                port: 5672,
                user: "guest".into(),
                password: "guest".into(),
            })
            .await
            .unwrap(),
        ));
        DefaultHealthChecker { db, redis, rabbit }
    }

    #[tokio::test]
    async fn test_check_postgres_returns_status() {
        let checker = make_checker().await;
        let result = checker.check_postgres().await;
        assert!(result.status == "up" || result.status == "down");
    }

    #[tokio::test]
    async fn test_check_redis_returns_status() {
        let checker = make_checker().await;
        let result = checker.check_redis().await;
        assert!(result.status == "down" || result.status == "up");
    }

    #[tokio::test]
    async fn test_check_rabbitmq_disabled() {
        let checker = make_checker().await;
        let result = checker.check_rabbitmq().await;
        assert_eq!(result.status, "disabled");
    }
}
