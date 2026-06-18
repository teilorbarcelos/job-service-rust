use lapin::{Connection, ConnectionProperties, ConnectionState};
use tracing::info;

use crate::core::errors::AppError;
use crate::shared::config::MessagingConfig;

pub struct MessagingProvider {
    connection: Option<Connection>,
    pub enabled: bool,
}

impl MessagingProvider {
    pub async fn connect(config: &MessagingConfig) -> Result<Self, AppError> {
        if !config.enabled {
            info!("Messaging disabled");
            return Ok(Self {
                connection: None,
                enabled: false,
            });
        }

        let addr = format!(
            "amqp://{}:{}@{}:{}/default",
            config.user, config.password, config.host, config.port
        );

        let connection = Connection::connect(&addr, ConnectionProperties::default())
            .await
            .map_err(|e| AppError::Connection(format!("RabbitMQ: {}", e)))?;

        info!("RabbitMQ connected");
        Ok(Self {
            connection: Some(connection),
            enabled: true,
        })
    }

    pub fn is_open(&self) -> bool {
        self.connection
            .as_ref()
            .map(|c| c.status().state() == ConnectionState::Connected)
            .unwrap_or(false)
    }

    pub async fn close(&mut self) {
        if let Some(conn) = self.connection.take() {
            conn.close(0, "").await.ok();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_connect_disabled() {
        let config = MessagingConfig {
            enabled: false,
            host: "localhost".into(),
            port: 5672,
            user: "guest".into(),
            password: "guest".into(),
        };
        let provider = MessagingProvider::connect(&config).await.unwrap();
        assert!(!provider.enabled);
        assert!(!provider.is_open());
    }

    #[tokio::test]
    async fn test_close_no_connection() {
        let mut provider = MessagingProvider {
            connection: None,
            enabled: true,
        };
        provider.close().await;
        assert!(!provider.is_open());
    }

    #[tokio::test]
    async fn test_connect_with_bad_host() {
        let config = MessagingConfig {
            enabled: true,
            host: "192.0.2.1".into(),
            port: 5672,
            user: "guest".into(),
            password: "guest".into(),
        };
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            MessagingProvider::connect(&config),
        )
        .await;
        match result {
            Ok(Ok(_)) => panic!("Expected error, got success"),
            Ok(Err(_)) => {} // Expected: connection refused
            Err(_) => {} // Timeout is acceptable
        }
    }

    #[tokio::test]
    async fn test_is_open_no_connection() {
        let provider = MessagingProvider {
            connection: None,
            enabled: true,
        };
        assert!(!provider.is_open());
    }
}
