use crate::errors::AppError;
use async_trait::async_trait;
use std::env;

pub struct GcsStorageService {
    client: Option<google_cloud_storage::client::Client>,
    bucket: String,
}

impl GcsStorageService {
    pub async fn new() -> Result<Self, AppError> {
        let bucket = env::var("GCS_BUCKET").unwrap_or_else(|_| "my-gcs-bucket".to_string());
        
        let client = if env::var("GOOGLE_APPLICATION_CREDENTIALS").is_ok() || env::var("GOOGLE_APPLICATION_CREDENTIALS_JSON").is_ok() {
            let config = google_cloud_storage::client::ClientConfig::default()
                .with_auth()
                .await
                .map_err(|e| AppError::Internal(format!("Failed to create GCS client config: {}", e)))?;
            Some(google_cloud_storage::client::Client::new(config))
        } else {
            tracing::warn!("GCS credentials not found. GcsStorageService initialized in OFFLINE/MOCK mode.");
            None
        };

        Ok(Self { client, bucket })
    }
}

#[async_trait]
impl crate::infra::storage::StorageService for GcsStorageService {
    async fn upload(&self, file_name: &str, data: &[u8]) -> Result<String, AppError> {
        let unique_id = uuid::Uuid::new_v4().to_string();
        let object_name = format!("{}_{}", unique_id, file_name);

        if let Some(client) = &self.client {
            use google_cloud_storage::http::objects::upload::{UploadObjectRequest, UploadType};
            
            client
                .upload_object(
                    &UploadObjectRequest {
                        bucket: self.bucket.clone(),
                        ..Default::default()
                    },
                    data.to_vec(),
                    &UploadType::Simple(google_cloud_storage::http::objects::upload::Media {
                        name: object_name.clone().into(),
                        content_type: "application/octet-stream".into(),
                        content_length: Some(data.len() as u64),
                    }),
                )
                .await
                .map_err(|e| AppError::Internal(format!("Failed to upload to GCS: {}", e)))?;
            
            Ok(format!("https://storage.googleapis.com/{}/{}", self.bucket, object_name))
        } else {
            tracing::info!("[GCS MOCK UPLOAD] Object: {}, Size: {} bytes", object_name, data.len());
            Ok(format!("https://storage.googleapis.com/{}/{}", self.bucket, object_name))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::storage::StorageService;

    static TEST_MUTEX: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();

    fn get_lock() -> std::sync::MutexGuard<'static, ()> {
        TEST_MUTEX.get_or_init(|| std::sync::Mutex::new(())).lock().unwrap()
    }

    #[tokio::test]
    async fn test_gcs_upload_offline() {
        let _guard = get_lock();
        let _cred_guard = std::env::var("GOOGLE_APPLICATION_CREDENTIALS").ok();
        let _json_guard = std::env::var("GOOGLE_APPLICATION_CREDENTIALS_JSON").ok();
        std::env::remove_var("GOOGLE_APPLICATION_CREDENTIALS");
        std::env::remove_var("GOOGLE_APPLICATION_CREDENTIALS_JSON");

        let service = GcsStorageService::new().await.unwrap();
        let content = b"hello gcs storage";
        let res = service.upload("test_gcs.txt", content).await;
        assert!(res.is_ok());

        let url = res.unwrap();
        assert!(url.contains("test_gcs.txt"));
        assert!(url.contains("my-gcs-bucket"));

        if let Some(cred) = _cred_guard {
            std::env::set_var("GOOGLE_APPLICATION_CREDENTIALS", cred);
        }
        if let Some(json) = _json_guard {
            std::env::set_var("GOOGLE_APPLICATION_CREDENTIALS_JSON", json);
        }
    }

    #[derive(Debug)]
    struct MockTokenSource;

    #[async_trait::async_trait]
    impl google_cloud_token::TokenSource for MockTokenSource {
        async fn token(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
            Ok("dummy-token".to_string())
        }
    }

    #[derive(Debug)]
    struct MockTokenSourceProvider;

    impl google_cloud_token::TokenSourceProvider for MockTokenSourceProvider {
        fn token_source(&self) -> std::sync::Arc<dyn google_cloud_token::TokenSource> {
            std::sync::Arc::new(MockTokenSource)
        }
    }

    #[tokio::test]
    async fn test_gcs_upload_online_error_handling() {
        let _guard = get_lock();
        let mut config = google_cloud_storage::client::ClientConfig::default();
        config.token_source_provider = Box::new(MockTokenSourceProvider);
        let client = google_cloud_storage::client::Client::new(config);
        let service = GcsStorageService {
            client: Some(client),
            bucket: "dummy-bucket-that-does-not-exist".to_string(),
        };

        let res = service.upload("test_error.txt", b"error test").await;
        assert!(res.is_err());
        let err = res.unwrap_err();
        assert!(err.message().contains("Failed to upload to GCS"));
    }

    #[tokio::test]
    async fn test_gcs_new_with_credentials_error() {
        let _guard = get_lock();
        std::env::set_var("GOOGLE_APPLICATION_CREDENTIALS", "non_existent_file.json");
        let res = GcsStorageService::new().await;
        assert!(res.is_err());
        std::env::remove_var("GOOGLE_APPLICATION_CREDENTIALS");
    }
}
