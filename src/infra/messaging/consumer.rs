use super::connection::MessagingProvider;
use crate::errors::AppError;
use lapin::{
    options::{BasicAckOptions, BasicConsumeOptions},
    types::FieldTable,
};
use serde::de::DeserializeOwned;
use tracing::error;

impl MessagingProvider {
    pub async fn subscribe<T, F, Fut>(&self, queue: &str, callback: F) -> Result<(), AppError>
    where
        T: DeserializeOwned + Send + 'static,
        F: Fn(T) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let channel = match self.get_channel_and_assert_queue(queue).await? {
            Some(c) => c,
            None => return Ok(()),
        };

        use futures_util::stream::StreamExt;

        let mut consumer = channel
            .basic_consume(
                queue,
                "",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await
            .map_err(|e| AppError::Internal(format!("Failed to basic_consume: {}", e)))?;

        tokio::spawn(async move {
            while let Some(delivery) = consumer.next().await {
                match delivery {
                    Ok(delivery) => match serde_json::from_slice::<T>(&delivery.data) {
                        Ok(content) => {
                            callback(content).await;
                            if let Err(e) = delivery.ack(BasicAckOptions::default()).await {
                                error!("[RabbitMQ] Failed to ack message: {}", e);
                            }
                        }
                        Err(e) => {
                            error!("[RabbitMQ] Failed to deserialize message: {}", e);
                            if let Err(ack_err) = delivery.ack(BasicAckOptions::default()).await {
                                error!("[RabbitMQ] Failed to ack corrupt message: {}", ack_err);
                            }
                        }
                    },
                    Err(e) => {
                        error!("[RabbitMQ] Error in consumer stream: {}", e);
                    }
                }
            }
        });

        Ok(())
    }
}
