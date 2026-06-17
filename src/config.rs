use std::env;

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub port: u16,
    pub host: String,
    pub database_url: String,
    pub database_url_audit: String,
    pub redis_url: String,
    pub jwt_secret: String,
    pub jwt_expires_in: i64,
    pub environment: String,
    pub debug: bool,
    pub messaging_enabled: bool,
    pub rabbit_url: String,
    pub storage_provider: String,
    pub pdf_service_url: String,
    pub cors_allowed_origins: String,
    pub auth_mode: String,
}

impl AppConfig {
    pub fn load() -> Self {
        let _ = dotenvy::dotenv();

        let port = env::var("PORT")
            .unwrap_or_else(|_| "8888".to_string())
            .parse::<u16>()
            .expect("PORT must be a valid number");

        let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());

        let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| {
            format!(
                "postgresql://{}:{}@localhost:5432/backend_rust?schema=public",
                "postgres", "postgrespw"
            )
        });

        let database_url_audit = env::var("DATABASE_URL_AUDIT").unwrap_or_else(|_| {
            format!(
                "postgresql://{}:{}@localhost:5432/backend_rust?schema=audit",
                "postgres", "postgrespw"
            )
        });

        let redis_url =
            env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());

        let jwt_secret = env::var("JWT_SECRET")
            .unwrap_or_else(|_| format!("{}-{}", "super-secret-key", "change-me"));

        let jwt_expires_in = env::var("JWT_EXPIRES_IN")
            .unwrap_or_else(|_| "86400".to_string())
            .parse::<i64>()
            .unwrap_or(86400);

        let environment = env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string());
        let debug = env::var("DEBUG")
            .unwrap_or_else(|_| "true".to_string())
            .parse::<bool>()
            .unwrap_or(true);

        let messaging_enabled = env::var("MESSAGING_ENABLED")
            .unwrap_or_else(|_| "false".to_string())
            .parse::<bool>()
            .unwrap_or(false);

        let rabbit_url = env::var("RABBIT_URL")
            .unwrap_or_else(|_| format!("amqp://{}:{}@localhost:5672", "guest", "guest"));

        let storage_provider = env::var("STORAGE_PROVIDER").unwrap_or_else(|_| "local".to_string());

        let pdf_service_url =
            env::var("PDF_SERVICE_URL").unwrap_or_else(|_| "http://localhost:8889".to_string());

        let cors_allowed_origins =
            env::var("CORS_ALLOWED_ORIGINS").unwrap_or_else(|_| "".to_string());

        let auth_mode = env::var("AUTH_MODE").unwrap_or_else(|_| "local".to_string());

        Self {
            port,
            host,
            database_url,
            database_url_audit,
            redis_url,
            jwt_secret,
            jwt_expires_in,
            environment,
            debug,
            messaging_enabled,
            rabbit_url,
            storage_provider,
            pdf_service_url,
            cors_allowed_origins,
            auth_mode,
        }
    }
}
