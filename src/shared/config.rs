use crate::core::errors::AppError;

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub driver: String,
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct RedisConfig {
    pub host: String,
    pub port: u16,
    pub password: String,
    pub db: i64,
}

#[derive(Debug, Clone)]
pub struct MessagingConfig {
    pub enabled: bool,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
}

#[derive(Debug, Clone)]
pub struct JobsConfig {
    pub health_check_cron: String,
    pub health_check_enabled: bool,
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub app_env: String,
    pub app_debug: bool,
    pub log_level: String,
    pub shutdown_timeout_ms: u64,
    pub job_execution_timeout_ms: u64,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub messaging: MessagingConfig,
    pub jobs: JobsConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            app_env: "testing".into(),
            app_debug: false,
            log_level: "error".into(),
            shutdown_timeout_ms: 30000,
            job_execution_timeout_ms: 300000,
            database: DatabaseConfig { driver: "sqlite".into(), url: "sqlite::memory:".into() },
            redis: RedisConfig { host: "localhost".into(), port: 6379, password: "".into(), db: 0 },
            messaging: MessagingConfig { enabled: false, host: "localhost".into(), port: 5672, user: "guest".into(), password: "guest".into() },
            jobs: JobsConfig { health_check_cron: "*/1 * * * * *".into(), health_check_enabled: true },
        }
    }
}

fn get_env(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

fn get_env_int(key: &str, default: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn get_env_bool(key: &str, default: bool) -> bool {
    std::env::var(key)
        .ok()
        .map(|v| matches!(v.to_lowercase().as_str(), "true" | "1" | "yes"))
        .unwrap_or(default)
}

fn get_env_int_u16(key: &str, default: u16) -> u16 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

pub fn load_config() -> Result<AppConfig, AppError> {
    if dotenvy::dotenv().is_err() {
        // .env file is optional
    }

    let db_url = get_env("DATABASE_URL", "sqlite::memory:");
    let db_driver = if db_url.starts_with("postgres") {
        "postgres"
    } else {
        "sqlite"
    };

    let messaging_enabled = get_env_bool("MESSAGING_ENABLED", false);
    let health_check_enabled = get_env_bool("HEALTH_CHECK_ENABLED", true);

    Ok(AppConfig {
        app_env: get_env("APP_ENV", "development"),
        app_debug: get_env_bool("APP_DEBUG", false),
        log_level: get_env("LOG_LEVEL", "info"),
        shutdown_timeout_ms: get_env_int("SHUTDOWN_TIMEOUT_MS", 30000),
        job_execution_timeout_ms: get_env_int("JOB_EXECUTION_TIMEOUT_MS", 300000),
        database: DatabaseConfig {
            driver: db_driver.to_string(),
            url: db_url,
        },
        redis: RedisConfig {
            host: get_env("REDIS_HOST", "localhost"),
            port: get_env_int_u16("REDIS_PORT", 6379),
            password: get_env("REDIS_PASSWORD", ""),
            db: get_env_int("REDIS_DB", 0) as i64,
        },
        messaging: MessagingConfig {
            enabled: messaging_enabled,
            host: get_env("RABBIT_HOST", "localhost"),
            port: get_env_int_u16("RABBIT_PORT", 5672),
            user: get_env("RABBIT_USER", "guest"),
            password: get_env("RABBIT_PASSWORD", "guest"),
        },
        jobs: JobsConfig {
            health_check_cron: get_env("HEALTH_CHECK_CRON", "*/1 * * * *"),
            health_check_enabled,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_env<T>(key: &str, value: &str, f: impl FnOnce() -> T) -> T {
        let prev = std::env::var(key).ok();
        std::env::set_var(key, value);
        let result = f();
        match prev {
            Some(v) => std::env::set_var(key, v),
            None => std::env::remove_var(key),
        }
        result
    }

    #[test]
    fn test_load_config_defaults() {
        with_env("DATABASE_URL", "sqlite::memory:", || {
            let cfg = load_config().unwrap();
            assert_eq!(cfg.database.driver, "sqlite");
        });
        with_env("HEALTH_CHECK_CRON", "*/5 * * * * *", || {
            let cfg = load_config().unwrap();
            assert_eq!(cfg.jobs.health_check_cron, "*/5 * * * * *");
        });
    }

    #[test]
    fn test_load_config_with_postgres() {
        with_env("DATABASE_URL", "postgres://user:pass@localhost/db", || {
            let cfg = load_config().unwrap();
            assert_eq!(cfg.database.driver, "postgres");
        });
    }

    #[test]
    fn test_load_config_messaging_enabled() {
        with_env("MESSAGING_ENABLED", "true", || {
            let cfg = load_config().unwrap();
            assert!(cfg.messaging.enabled);
        });
    }

    #[test]
    fn test_default_config() {
        let cfg = AppConfig::default();
        assert_eq!(cfg.app_env, "testing");
        assert_eq!(cfg.log_level, "error");
        assert_eq!(cfg.database.driver, "sqlite");
        assert!(cfg.jobs.health_check_enabled);
    }

    #[test]
    fn test_get_env_functions() {
        std::env::set_var("TEST_ENV_VAR", "42");
        assert_eq!(get_env("TEST_ENV_VAR", "0"), "42");
        assert_eq!(get_env("NONEXISTENT", "default"), "default");
        assert_eq!(get_env_int("TEST_ENV_VAR", 0), 42);
        assert_eq!(get_env_int("NONEXISTENT", 99), 99);
        assert!(!get_env_bool("TEST_ENV_VAR", false));
        std::env::remove_var("TEST_ENV_VAR");
    }

    #[test]
    fn test_get_env_bool_true_values() {
        std::env::set_var("BOOL_TEST", "true");
        assert!(get_env_bool("BOOL_TEST", false));
        std::env::set_var("BOOL_TEST", "1");
        assert!(get_env_bool("BOOL_TEST", false));
        std::env::set_var("BOOL_TEST", "yes");
        assert!(get_env_bool("BOOL_TEST", false));
        std::env::remove_var("BOOL_TEST");
    }

    #[test]
    fn test_get_env_int_u16() {
        std::env::set_var("PORT_TEST", "8080");
        assert_eq!(get_env_int_u16("PORT_TEST", 0), 8080);
        assert_eq!(get_env_int_u16("NONEXISTENT", 9999), 9999);
        std::env::remove_var("PORT_TEST");
    }
}
