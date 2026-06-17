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
    } else if db_url.starts_with("sqlite") {
        "sqlite"
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
