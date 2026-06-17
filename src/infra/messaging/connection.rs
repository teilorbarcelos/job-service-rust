use crate::config::AppConfig;
use crate::errors::AppError;
use lapin::{Channel, Connection, ConnectionProperties};
use std::sync::Arc;
use std::sync::OnceLock;
use tracing::{error, info};

pub static MESSAGING_PROVIDER: OnceLock<MessagingProvider> = OnceLock::new();

#[derive(Clone)]
pub struct MessagingProvider {
    pub(crate) connection: Option<Arc<Connection>>,
    pub(crate) channel: Option<Channel>,
    pub(crate) enabled: bool,
}

impl MessagingProvider {
    pub async fn init(config: &AppConfig) -> Result<(), AppError> {
        let enabled = config.messaging_enabled;

        let (connection, channel) = if enabled {
            info!("[RabbitMQ] Connecting to {}...", config.rabbit_url);
            let conn = Connection::connect(&config.rabbit_url, ConnectionProperties::default())
                .await
                .map_err(|e| AppError::Internal(format!("Failed to connect to RabbitMQ: {}", e)))?;

            let chan = conn.create_channel().await;

            #[cfg(test)]
            let chan = if config.rabbit_url.contains("FORCE_CHANNEL_ERR") {
                Err(lapin::Error::ChannelsLimitReached)
            } else {
                chan
            };

            let chan = chan.map_err(|e| {
                AppError::Internal(format!("Failed to create RabbitMQ channel: {}", e))
            })?;

            info!("[RabbitMQ] Connected successfully");
            (Some(Arc::new(conn)), Some(chan))
        } else {
            (None, None)
        };

        let provider = Self {
            connection,
            channel,
            enabled,
        };

        if MESSAGING_PROVIDER.set(provider).is_err() {
            error!("[RabbitMQ] MESSAGING_PROVIDER was already initialized");
        }

        Ok(())
    }

    pub fn get() -> &'static Self {
        MESSAGING_PROVIDER
            .get()
            .expect("MessagingProvider is not initialized")
    }

    pub async fn disconnect(&self) -> Result<(), AppError> {
        if let Some(channel) = &self.channel {
            let _ = channel.close(0, "Disconnecting").await;
        }
        if let Some(connection) = &self.connection {
            let _ = connection.close(0, "Disconnecting").await;
        }
        Ok(())
    }

    pub(crate) async fn get_channel_and_assert_queue(
        &self,
        queue: &str,
    ) -> Result<Option<&Channel>, AppError> {
        let channel = match &self.channel {
            Some(c) => c,
            None => {
                if self.enabled {
                    return Err(AppError::Internal(
                        "RabbitMQ channel not initialized".to_string(),
                    ));
                }
                return Ok(None);
            }
        };

        channel
            .queue_declare(
                queue,
                lapin::options::QueueDeclareOptions {
                    durable: true,
                    ..Default::default()
                },
                lapin::types::FieldTable::default(),
            )
            .await
            .map_err(|e| AppError::Internal(format!("Failed to assert queue: {}", e)))?;

        Ok(Some(channel))
    }
}
