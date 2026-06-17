use crate::errors::AppError;
use async_trait::async_trait;
use std::env;

pub struct AzureStorageService {
    client: Option<azure_storage_blobs::prelude::BlobServiceClient>,
    container: String,
}

impl AzureStorageService {
    pub async fn new() -> Result<Self, AppError> {
        let container = env::var("AZURE_STORAGE_CONTAINER").unwrap_or_else(|_| "my-container".to_string());
        
        let client = if let Ok(connection_string) = env::var("AZURE_STORAGE_CONNECTION_STRING") {
            let conn_str = azure_storage::ConnectionString::new(&connection_string)
                .map_err(|e| AppError::Internal(format!("Failed to parse connection string: {}", e)))?;
            let account = conn_str.account_name
                .ok_or_else(|| AppError::Internal("Missing AccountName in Azure connection string".to_string()))?;
            let credentials = conn_str.storage_credentials()
                .map_err(|e| AppError::Internal(format!("Failed to extract storage credentials: {}", e)))?;
            
            let client = azure_storage_blobs::prelude::BlobServiceClient::new(account, credentials);
            Some(client)
        } else {
            tracing::warn!("AZURE_STORAGE_CONNECTION_STRING not found. AzureStorageService initialized in OFFLINE/MOCK mode.");
            None
        };

        Ok(Self { client, container })
    }
}

#[async_trait]
impl crate::infra::storage::StorageService for AzureStorageService {
    async fn upload(&self, file_name: &str, data: &[u8]) -> Result<String, AppError> {
        let unique_id = uuid::Uuid::new_v4().to_string();
        let blob_name = format!("{}_{}", unique_id, file_name);

        if let Some(client) = &self.client {
            client
                .container_client(&self.container)
                .blob_client(&blob_name)
                .put_block_blob(data.to_vec())
                .await
                .map_err(|e| AppError::Internal(format!("Failed to upload to Azure Blob: {}", e)))?;
            
            let account = client.account();
            Ok(format!("https://{}.blob.core.windows.net/{}/{}", account, self.container, blob_name))
        } else {
            tracing::info!("[Azure MOCK UPLOAD] Blob: {}, Size: {} bytes", blob_name, data.len());
            Ok(format!("https://mock-account.blob.core.windows.net/{}/{}", self.container, blob_name))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infra::storage::StorageService;

    static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[tokio::test]
    async fn test_azure_upload_offline() {
        let _guard = TEST_LOCK.lock().unwrap();
        std::env::remove_var("AZURE_STORAGE_CONNECTION_STRING");
        std::env::remove_var("AZURE_STORAGE_CONTAINER");

        let service = AzureStorageService::new().await.unwrap();
        let content = b"hello azure storage";
        let res = service.upload("test_azure.txt", content).await;
        assert!(res.is_ok());

        let url = res.unwrap();
        assert!(url.contains("test_azure.txt"));
        assert!(url.contains("my-container"));
    }

    #[tokio::test]
    async fn test_azure_upload_online_error_handling() {
        let _guard = TEST_LOCK.lock().unwrap();
        let dummy_conn = "DefaultEndpointsProtocol=https;AccountName=dummyaccount;AccountKey=ZHVtbXlrZXk=;EndpointSuffix=core.windows.net";
        std::env::set_var("AZURE_STORAGE_CONNECTION_STRING", dummy_conn);
        std::env::set_var("AZURE_STORAGE_CONTAINER", "dummy-container");

        let service = AzureStorageService::new().await.unwrap();
        assert!(service.client.is_some());

        let res = service.upload("test_error.txt", b"error test").await;
        assert!(res.is_err());
        let err = res.unwrap_err();
        assert!(err.message().contains("Failed to upload to Azure Blob"));

        std::env::remove_var("AZURE_STORAGE_CONNECTION_STRING");
        std::env::remove_var("AZURE_STORAGE_CONTAINER");
    }
}
