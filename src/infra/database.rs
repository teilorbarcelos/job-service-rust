use sqlx::postgres::PgPoolOptions;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Pool, Postgres, Sqlite};
use tracing::info;

use crate::core::errors::AppError;
use crate::shared::config::DatabaseConfig;

pub enum DatabasePool {
    Postgres(Pool<Postgres>),
    Sqlite(Pool<Sqlite>),
}

impl DatabasePool {
    pub async fn connect(config: &DatabaseConfig) -> Result<Self, AppError> {
        match config.driver.as_str() {
            "postgres" => {
                let pool = PgPoolOptions::new()
                    .max_connections(5)
                    .connect(&config.url)
                    .await
                    .map_err(|e| AppError::Connection(format!("PostgreSQL: {}", e)))?;
                info!("Database connected (PostgreSQL)");
                Ok(DatabasePool::Postgres(pool))
            }
            _ => {
                let pool = SqlitePoolOptions::new()
                    .max_connections(1)
                    .connect(&config.url)
                    .await
                    .map_err(|e| AppError::Connection(format!("SQLite: {}", e)))?;
                info!("Database connected (SQLite)");
                Ok(DatabasePool::Sqlite(pool))
            }
        }
    }

    pub async fn ping(&self) -> bool {
        match self {
            DatabasePool::Postgres(pool) => sqlx::query("SELECT 1").execute(pool).await.is_ok(),
            DatabasePool::Sqlite(pool) => sqlx::query("SELECT 1").execute(pool).await.is_ok(),
        }
    }

    pub async fn close(&self) {
        match self {
            DatabasePool::Postgres(pool) => pool.close().await,
            DatabasePool::Sqlite(pool) => pool.close().await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connect_sqlite_memory() {
        let config = DatabaseConfig {
            driver: "sqlite".into(),
            url: "sqlite::memory:".into(),
        };
        let pool = DatabasePool::connect(&config).await;
        assert!(pool.is_ok());
    }

    #[tokio::test]
    async fn test_ping_sqlite() {
        let config = DatabaseConfig {
            driver: "sqlite".into(),
            url: "sqlite::memory:".into(),
        };
        let pool = DatabasePool::connect(&config).await.unwrap();
        assert!(pool.ping().await);
    }

    #[tokio::test]
    async fn test_close_sqlite() {
        let config = DatabaseConfig {
            driver: "sqlite".into(),
            url: "sqlite::memory:".into(),
        };
        let pool = DatabasePool::connect(&config).await.unwrap();
        pool.close().await;
    }
}
