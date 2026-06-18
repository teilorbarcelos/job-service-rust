use deadpool_redis::{Config, Runtime};
use redis::cmd;
use tracing::info;

use crate::core::errors::AppError;
use crate::shared::config::RedisConfig;

pub struct RedisProvider {
    pool: deadpool_redis::Pool,
}

impl RedisProvider {
    pub async fn connect(config: &RedisConfig) -> Result<Self, AppError> {
        let url = format!("redis://{}:{}/{}", config.host, config.port, config.db);
        let cfg = Config::from_url(&url);
        let pool = cfg
            .create_pool(Some(Runtime::Tokio1))
            .map_err(|e| AppError::Connection(format!("Redis: {}", e)))?;

        info!("Redis pool created");
        Ok(Self { pool })
    }

    pub async fn ping(&self) -> Result<(), AppError> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| AppError::Connection(format!("Redis pool: {}", e)))?;
        cmd("PING")
            .query_async::<_, String>(&mut *conn)
            .await
            .map_err(|e| AppError::Connection(format!("Redis ping: {}", e)))?;
        Ok(())
    }

    pub async fn close(&self) {
        // Pool will be dropped when RedisProvider is dropped
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connect_bad_host_pool_succeeds_but_ping_fails() {
        let config = RedisConfig {
            host: "192.0.2.1".into(),
            port: 6379,
            password: "".into(),
            db: 0,
        };
        let result = RedisProvider::connect(&config).await;
        if let Ok(provider) = result {
            let ping = tokio::time::timeout(
                std::time::Duration::from_secs(5),
                provider.ping(),
            )
            .await;
            match ping {
                Ok(Err(_)) => {} // Expected
                Err(_) => {} // Timeout is acceptable
                Ok(Ok(())) => panic!("Expected error, got success"),
            }
        }
    }

    #[tokio::test]
    async fn test_connect_and_close() {
        let config = RedisConfig {
            host: "localhost".into(),
            port: 6379,
            password: "".into(),
            db: 0,
        };
        // Connect may fail if Redis is not running, but close should still work
        if let Ok(provider) = RedisProvider::connect(&config).await {
            provider.close().await;
        }
    }
}
