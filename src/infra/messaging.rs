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
