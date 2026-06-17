use crate::errors::AppError;
use async_trait::async_trait;
use std::fs;
use std::path::Path;

pub struct LocalStorageService;

impl LocalStorageService {
    pub fn new() -> Self {
        let path = Path::new("uploads");
        if !path.exists() {
            let _ = fs::create_dir_all(path);
        }
        Self
    }
}

impl Default for LocalStorageService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl super::StorageService for LocalStorageService {
    async fn upload(&self, file_name: &str, data: &[u8]) -> Result<String, AppError> {
        let unique_id = uuid::Uuid::new_v4().to_string();
        let safe_name = format!("{}_{}", unique_id, file_name);
        let path_str = format!("uploads/{}", safe_name);
        let path = Path::new(&path_str);

        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).map_err(|e| {
                    AppError::Internal(format!("Failed to create storage directory: {}", e))
                })?;
            }
        }

        fs::write(path, data).map_err(|e| {
            AppError::Internal(format!("Failed to write file to local storage: {}", e))
        })?;

        Ok(format!("/uploads/{}", safe_name))
    }
}
#[cfg(test)]
pub static TEST_MUTEX: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();

#[cfg(test)]
pub fn get_lock() -> std::sync::MutexGuard<'static, ()> {
    TEST_MUTEX
        .get_or_init(|| std::sync::Mutex::new(()))
        .lock()
        .unwrap()
}

#[cfg(test)]
#[allow(clippy::await_holding_lock)]
mod tests {
    use super::*;
    use crate::infra::storage::StorageService;

    #[tokio::test]
    #[allow(clippy::default_constructed_unit_structs)]
    async fn test_local_upload_success() {
        let _guard = get_lock();
        let _default_service = LocalStorageService::default();
        let service = LocalStorageService::new();
        let content = b"hello local storage";
        let res = service.upload("test.txt", content).await;
        assert!(res.is_ok());

        let url = res.unwrap();
        assert!(url.starts_with("/uploads/"));
        assert!(url.contains("test.txt"));

        let path_on_disk = url.trim_start_matches('/');
        assert!(Path::new(path_on_disk).exists());

        let disk_content = fs::read_to_string(path_on_disk).unwrap();
        assert_eq!(disk_content, "hello local storage");

        let _ = fs::remove_file(path_on_disk);
    }

    #[tokio::test]
    async fn test_local_upload_errors() {
        let _guard = get_lock();
        let temp_dir_name = "uploads_backup_test";
        if Path::new(temp_dir_name).exists() {
            let _ = fs::remove_dir_all(temp_dir_name);
            let _ = fs::remove_file(temp_dir_name);
        }

        let mut uploads_existed = Path::new("uploads").exists();
        if uploads_existed {
            if !Path::new("uploads").is_dir() {
                let _ = fs::remove_file("uploads");
                uploads_existed = false;
            } else {
                fs::rename("uploads", temp_dir_name).unwrap();
            }
        }

        {
            let _service = LocalStorageService::new();
            assert!(Path::new("uploads").exists());
            fs::remove_dir("uploads").unwrap();
        }

        {
            let service = LocalStorageService::new();
            let long_name = format!("{}/test.txt", "a/".repeat(2500));
            let res = service.upload(&long_name, b"hello").await;
            assert!(res.is_err());
            let err = res.unwrap_err();
            assert!(err.message().contains("Failed to create storage directory"));
        }

        {
            if Path::new("uploads").exists() {
                let _ = fs::remove_dir_all("uploads");
            }
            fs::write("uploads", "plain file").unwrap();
            let service = LocalStorageService::new();
            let res = service.upload("test.txt", b"hello").await;
            assert!(res.is_err());
            let err = res.unwrap_err();
            assert!(err
                .message()
                .contains("Failed to write file to local storage"));
            fs::remove_file("uploads").unwrap();
        }

        {
            fs::create_dir_all("uploads").unwrap();
            let mut perms = fs::metadata("uploads").unwrap().permissions();
            perms.set_readonly(true);
            let _ = fs::set_permissions("uploads", perms.clone());

            let service = LocalStorageService::new();
            let res = service.upload("test.txt", b"hello").await;
            assert!(res.is_err());
            let err = res.unwrap_err();
            assert!(err
                .message()
                .contains("Failed to write file to local storage"));

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = fs::set_permissions("uploads", PermissionsExt::from_mode(0o777));
            }
            #[cfg(not(unix))]
            {
                perms.set_readonly(false);
                let _ = fs::set_permissions("uploads", perms);
            }
            let _ = fs::remove_dir_all("uploads");
        }

        if uploads_existed {
            fs::rename(temp_dir_name, "uploads").unwrap();
        }
    }
}
