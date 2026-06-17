use crate::config::AppConfig;
use crate::errors::AppError;
use async_trait::async_trait;
use std::sync::Arc;
use std::sync::OnceLock;
use tracing::{error, info};

pub mod local;

#[async_trait]
pub trait StorageService: Send + Sync + 'static {
    async fn upload(&self, file_name: &str, data: &[u8]) -> Result<String, AppError>;
}

pub static STORAGE_PROVIDER: OnceLock<StorageProvider> = OnceLock::new();

#[derive(Clone)]
pub struct StorageProvider {
    service: Arc<dyn StorageService>,
}

impl StorageProvider {
    pub async fn init(config: &AppConfig) -> Result<(), AppError> {
        let service: Arc<dyn StorageService> = match config.storage_provider.as_str() {
            "local" | "" => {
                info!("[Storage] Initializing LocalStorageService...");
                Arc::new(local::LocalStorageService::new())
            }
            /* {{GENERATED_PROVIDERS}} */
            other => {
                return Err(AppError::Internal(format!(
                    "Provedor de storage '{}' desconhecido ou não implementado.",
                    other
                )));
            }
        };

        let provider = Self { service };

        if STORAGE_PROVIDER.set(provider).is_err() {
            error!("[Storage] STORAGE_PROVIDER was already initialized");
        }

        Ok(())
    }

    pub fn get() -> &'static Self {
        STORAGE_PROVIDER
            .get()
            .expect("StorageProvider is not initialized")
    }

    pub async fn upload(&self, file_name: &str, data: &[u8]) -> Result<String, AppError> {
        self.service.upload(file_name, data).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[allow(clippy::await_holding_lock)]
    async fn test_storage_provider_init_all() {
        let _guard = local::get_lock();
        let mut config = crate::config::AppConfig::load();

        config.storage_provider = "unknown_provider_name".to_string();
        let res = StorageProvider::init(&config).await;
        assert!(res.is_err());
        assert!(res
            .unwrap_err()
            .message()
            .contains("desconhecido ou não implementado"));

        config.storage_provider = "s3".to_string();
        let _ = StorageProvider::init(&config).await;

        config.storage_provider = "local".to_string();
        let _ = StorageProvider::init(&config).await;
        let _ = StorageProvider::init(&config).await;

        config.storage_provider = "gcs".to_string();
        let _ = StorageProvider::init(&config).await;

        let _env_guard = std::env::var("AZURE_STORAGE_CONNECTION_STRING").ok();
        std::env::remove_var("AZURE_STORAGE_CONNECTION_STRING");
        config.storage_provider = "azure".to_string();
        let _ = StorageProvider::init(&config).await;

        if let Some(conn) = _env_guard {
            std::env::set_var("AZURE_STORAGE_CONNECTION_STRING", conn);
        }
    }
}
