use super::connection::MessagingProvider;
use crate::errors::AppError;
use lapin::{options::BasicPublishOptions, BasicProperties};
use serde::Serialize;

impl MessagingProvider {
    pub async fn publish<T: Serialize>(&self, queue: &str, message: &T) -> Result<(), AppError> {
        let channel = match self.get_channel_and_assert_queue(queue).await? {
            Some(c) => c,
            None => return Ok(()),
        };

        let payload = serde_json::to_vec(message)
            .map_err(|e| AppError::Internal(format!("Failed to serialize message: {}", e)))?;

        channel
            .basic_publish(
                "",
                queue,
                BasicPublishOptions::default(),
                &payload,
                BasicProperties::default(),
            )
            .await
            .map_err(|e| AppError::Internal(format!("Failed to send publish command: {}", e)))?
            .await
            .map_err(|e| AppError::Internal(format!("Failed to confirm publish: {}", e)))?;

        Ok(())
    }
}
