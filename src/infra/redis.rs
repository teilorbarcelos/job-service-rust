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
