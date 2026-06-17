use crate::errors::AppError;
use async_trait::async_trait;
use std::env;

pub struct S3StorageService {
    client: Option<aws_sdk_s3::Client>,
    bucket: String,
}

impl S3StorageService {
    pub async fn new() -> Result<Self, AppError> {
        let bucket = env::var("AWS_S3_BUCKET").unwrap_or_else(|_| "my-bucket".to_string());
        
        let client = if env::var("AWS_ACCESS_KEY_ID").is_ok() && env::var("AWS_SECRET_ACCESS_KEY").is_ok() {
            let config = aws_config::defaults(aws_config::BehaviorVersion::latest()).load().await;
            Some(aws_sdk_s3::Client::new(&config))
        } else {
            tracing::warn!("AWS credentials not found. S3StorageService initialized in OFFLINE/MOCK mode.");
            None
        };

        Ok(Self { client, bucket })
    }
}

#[async_trait]
impl crate::infra::storage::StorageService for S3StorageService {
    async fn upload(&self, file_name: &str, data: &[u8]) -> Result<String, AppError> {
        let unique_id = uuid::Uuid::new_v4().to_string();
        let key = format!("{}_{}", unique_id, file_name);

        if let Some(client) = &self.client {
            client
                .put_object()
                .bucket(&self.bucket)
                .key(&key)
                .body(data.to_vec().into())
                .send()
                .await
                .map_err(|e| AppError::Internal(format!("Failed to upload to S3: {}", e)))?;
            
            Ok(format!("https://{}.s3.amazonaws.com/{}", self.bucket, key))
        } else {
            tracing::info!("[S3 MOCK UPLOAD] Key: {}, Size: {} bytes", key, data.len());
            Ok(format!("https://{}.s3.amazonaws.com/{}", self.bucket, key))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::storage::StorageService;

    static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[tokio::test]
    async fn test_s3_upload_offline() {
        let _guard = TEST_LOCK.lock().unwrap();
        let _key_guard = std::env::var("AWS_ACCESS_KEY_ID").ok();
        let _sec_guard = std::env::var("AWS_SECRET_ACCESS_KEY").ok();
        let _buc_guard = std::env::var("AWS_S3_BUCKET").ok();
        std::env::remove_var("AWS_ACCESS_KEY_ID");
        std::env::remove_var("AWS_SECRET_ACCESS_KEY");
        std::env::remove_var("AWS_S3_BUCKET");

        let service = S3StorageService::new().await.unwrap();
        let content = b"hello s3 storage";
        let res = service.upload("test_s3.txt", content).await;
        assert!(res.is_ok());

        let url = res.unwrap();
        assert!(url.contains("test_s3.txt"));
        assert!(url.contains("my-bucket"));

        if let Some(key) = _key_guard {
            std::env::set_var("AWS_ACCESS_KEY_ID", key);
        }
        if let Some(sec) = _sec_guard {
            std::env::set_var("AWS_SECRET_ACCESS_KEY", sec);
        }
        if let Some(buc) = _buc_guard {
            std::env::set_var("AWS_S3_BUCKET", buc);
        }
    }

    #[tokio::test]
    async fn test_s3_upload_online_error_handling() {
        let _guard = TEST_LOCK.lock().unwrap();
        let _key_guard = std::env::var("AWS_ACCESS_KEY_ID").ok();
        let _sec_guard = std::env::var("AWS_SECRET_ACCESS_KEY").ok();
        let _reg_guard = std::env::var("AWS_REGION").ok();
        let _buc_guard = std::env::var("AWS_S3_BUCKET").ok();

        std::env::set_var("AWS_ACCESS_KEY_ID", "dummy_key");
        std::env::set_var("AWS_SECRET_ACCESS_KEY", "dummy_secret");
        std::env::set_var("AWS_REGION", "us-east-1");
        std::env::set_var("AWS_S3_BUCKET", "dummy-bucket-name-that-should-not-exist");

        let service = S3StorageService::new().await.unwrap();
        assert!(service.client.is_some());

        let res = service.upload("test_error.txt", b"error test").await;
        assert!(res.is_err());
        let err = res.unwrap_err();
        assert!(err.message().contains("Failed to upload to S3"));

        if let Some(key) = _key_guard {
            std::env::set_var("AWS_ACCESS_KEY_ID", key);
        } else {
            std::env::remove_var("AWS_ACCESS_KEY_ID");
        }
        if let Some(sec) = _sec_guard {
            std::env::set_var("AWS_SECRET_ACCESS_KEY", sec);
        } else {
            std::env::remove_var("AWS_SECRET_ACCESS_KEY");
        }
        if let Some(reg) = _reg_guard {
            std::env::set_var("AWS_REGION", reg);
        } else {
            std::env::remove_var("AWS_REGION");
        }
        if let Some(buc) = _buc_guard {
            std::env::set_var("AWS_S3_BUCKET", buc);
        } else {
            std::env::remove_var("AWS_S3_BUCKET");
        }
    }
}
