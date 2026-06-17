use axum::{
    body::Body,
    http::{header, Request, Response, StatusCode},
    Router,
};
use backend_rust::{
    config::AppConfig,
    infra::{bootstrap::bootstrap_database, cache::Cache, database},
    middleware, modules,
};
use sea_orm::{ConnectionTrait, DatabaseConnection, Statement};
use testcontainers::{runners::AsyncRunner, ContainerAsync};
use testcontainers_modules::{postgres::Postgres, redis::Redis};
use tower::ServiceExt;

pub struct TestContext {
    pub db: DatabaseConnection,
    pub cache: Cache,
    pub config: AppConfig,
    pub router: Router,
    _postgres: ContainerAsync<Postgres>,
    _redis: ContainerAsync<Redis>,
}

impl TestContext {
    pub async fn new() -> Self {
        dotenvy::dotenv().ok();
        let mut config = AppConfig::load();

        let postgres_container = Postgres::default()
            .start()
            .await
            .expect("Failed to start Testcontainers Postgres");
        let pg_host = postgres_container
            .get_host()
            .await
            .expect("Failed to get Postgres host");
        let pg_port = postgres_container
            .get_host_port_ipv4(5432)
            .await
            .expect("Failed to get Postgres port");

        let redis_container = Redis::default()
            .start()
            .await
            .expect("Failed to start Testcontainers Redis");
        let redis_host = redis_container
            .get_host()
            .await
            .expect("Failed to get Redis host");
        let redis_port = redis_container
            .get_host_port_ipv4(6379)
            .await
            .expect("Failed to get Redis port");

        config.database_url = format!(
            "postgresql://postgres:postgres@{}:{}/postgres?schema=public",
            pg_host, pg_port
        );
        config.database_url_audit = format!(
            "postgresql://postgres:postgres@{}:{}/postgres?schema=audit",
            pg_host, pg_port
        );
        config.redis_url = format!("redis://{}:{}", redis_host, redis_port);
        config.jwt_expires_in = 3600;

        let db = database::connect(&config.database_url)
            .await
            .expect("Failed to connect to test Postgres database");

        let active_migrations: Vec<String> = backend_rust::migration::Migrator::migrations()
            .iter()
            .map(|m| m.name().to_string())
            .collect();

        let has_migrations_table = db.query_one(Statement::from_string(
            db.get_database_backend(),
            "SELECT EXISTS (SELECT FROM pg_tables WHERE schemaname = 'public' AND tablename = 'seaql_migrations');",
        )).await.ok().flatten().and_then(|row| row.try_get::<bool>("", "exists").ok()).unwrap_or(false);

        if has_migrations_table && !active_migrations.is_empty() {
            let quoted_active: Vec<String> = active_migrations
                .iter()
                .map(|m| format!("'{}'", m))
                .collect();
            let query = format!(
                "SELECT version FROM seaql_migrations WHERE version NOT IN ({})",
                quoted_active.join(", ")
            );
            if let Ok(stale_rows) = db
                .query_all(Statement::from_string(db.get_database_backend(), query))
                .await
            {
                for row in stale_rows {
                    if let Ok(version) = row.try_get::<String>("", "version") {
                        let parts: Vec<&str> = version.split("_create_").collect();
                        if parts.len() > 1 {
                            let table_name_part = parts[1].trim_end_matches("_table");
                            let _ = db
                                .execute(Statement::from_string(
                                    db.get_database_backend(),
                                    format!(
                                        "DROP TABLE IF EXISTS public.\"{}\" CASCADE;",
                                        table_name_part
                                    ),
                                ))
                                .await;

                            let pascal_table_name: String = table_name_part
                                .split('_')
                                .map(|word| {
                                    let mut chars = word.chars();
                                    match chars.next() {
                                        None => String::new(),
                                        Some(f) => {
                                            f.to_uppercase().collect::<String>() + chars.as_str()
                                        }
                                    }
                                })
                                .collect();
                            let _ = db
                                .execute(Statement::from_string(
                                    db.get_database_backend(),
                                    format!(
                                        "DROP TABLE IF EXISTS public.\"{}\" CASCADE;",
                                        pascal_table_name
                                    ),
                                ))
                                .await;
                        }

                        let _ = db
                            .execute(Statement::from_string(
                                db.get_database_backend(),
                                format!(
                                    "DELETE FROM seaql_migrations WHERE version = '{}'",
                                    version
                                ),
                            ))
                            .await;
                    }
                }
            }
        }

        use sea_orm_migration::MigratorTrait;
        backend_rust::migration::Migrator::up(&db, None)
            .await
            .expect("Failed to run migrations on test database");

        bootstrap_database(&db)
            .await
            .expect("Failed to bootstrap test database");

        let cache = Cache::new(&config.redis_url);

        let mut test_config = config.clone();
        test_config.messaging_enabled = false;
        test_config.environment = "development".to_string();
        test_config.storage_provider = "local".to_string();

        let _ = backend_rust::infra::messaging::MessagingProvider::init(&test_config).await;
        let _ = backend_rust::infra::storage::StorageProvider::init(&test_config).await;

        let api_router = modules::app_router(db.clone(), cache.clone(), test_config.clone());
        let obs_router = modules::observability::router(db.clone(), cache.clone());

        let router = Router::new()
            .merge(api_router)
            .merge(obs_router)
            .nest_service("/uploads", tower_http::services::ServeDir::new("uploads"))
            .layer(axum::middleware::from_fn_with_state(
                db.clone(),
                middleware::error_log::error_logging_middleware,
            ))
            .layer(axum::middleware::from_fn_with_state(
                db.clone(),
                middleware::audit::audit_middleware,
            ))
            .layer(axum::middleware::from_fn_with_state(
                cache.clone(),
                middleware::rate_limit::rate_limit_middleware,
            ))
            .layer(axum::middleware::from_fn(
                middleware::request_log::request_logging_middleware,
            ))
            .layer(axum::middleware::from_fn(
                modules::observability::track_metrics_middleware,
            ));

        Self {
            db,
            cache,
            config: test_config,
            router,
            _postgres: postgres_container,
            _redis: redis_container,
        }
    }

    pub async fn clear_database(&self) {
        let statements = vec![
            "TRUNCATE TABLE public.\"Product\", audit.tb_audit, audit.tb_error_log CASCADE;",
            "DELETE FROM public.\"RoleFeature\" WHERE id_role != 'administrator';",
            "DELETE FROM public.\"User\" WHERE email != 'admin@email.com';",
            "DELETE FROM public.\"Auth\" WHERE id != 'auth-admin-uuid-00000000000000000001';",
            "DELETE FROM public.\"Role\" WHERE id != 'administrator';",

            "UPDATE public.\"RoleFeature\" SET \"create\" = true, \"view\" = true, \"activate\" = true, \"delete\" = true WHERE id_role = 'administrator';",
            "UPDATE public.\"Role\" SET active = true, name = 'Administrador', description = 'Perfil com acesso total ao sistema', is_deleted = false, deleted_at = NULL WHERE id = 'administrator';",
            "UPDATE public.\"User\" SET active = true, is_deleted = false, deleted_at = NULL, name = 'Supreme Administrator' WHERE email = 'admin@email.com';",
            "UPDATE public.\"Auth\" SET active = true, is_deleted = false, deleted_at = NULL WHERE id = 'auth-admin-uuid-00000000000000000001';",
        ];

        for stmt in statements {
            let res = self
                .db
                .execute(Statement::from_string(
                    self.db.get_database_backend(),
                    stmt.to_string(),
                ))
                .await;
            if let Err(e) = res {
                eprintln!(
                    "Warning: database cleanup statement failed: {}. Query: {}",
                    e, stmt
                );
            }
        }
    }

    pub async fn reset_rate_limiter(&self) {
        let mut conn = self
            .cache
            .pool
            .get()
            .await
            .expect("Failed to connect to Redis for reset");
        let keys: Vec<String> = redis::cmd("KEYS")
            .arg("rate_limit:*")
            .query_async(&mut conn)
            .await
            .unwrap_or_default();
        let keys_no_underscore: Vec<String> = redis::cmd("KEYS")
            .arg("ratelimit:*")
            .query_async(&mut conn)
            .await
            .unwrap_or_default();
        let all_keys = [keys, keys_no_underscore].concat();
        if !all_keys.is_empty() {
            let mut del_cmd = redis::cmd("DEL");
            for key in &all_keys {
                del_cmd.arg(key);
            }
            let _: () = del_cmd.query_async(&mut conn).await.unwrap_or_default();
        }
    }
}

pub struct TestClient {
    router: Router,
    token: Option<String>,
}

impl TestClient {
    pub fn new(router: Router) -> Self {
        Self {
            router,
            token: None,
        }
    }

    pub fn set_token(&mut self, token: Option<String>) {
        self.token = token;
    }

    pub async fn request(
        &mut self,
        method: &str,
        uri: &str,
        body: Body,
        content_type: Option<&str>,
    ) -> (StatusCode, Response<Body>) {
        let mut req = Request::builder().method(method).uri(uri);

        if let Some(ref t) = self.token {
            req = req.header(header::AUTHORIZATION, format!("Bearer {}", t));
        }

        if let Some(ct) = content_type {
            req = req.header(header::CONTENT_TYPE, ct);
        }

        let req = req.body(body).unwrap();
        let resp = self.router.clone().oneshot(req).await.unwrap();
        (resp.status(), resp)
    }

    pub async fn request_with_headers(
        &mut self,
        method: &str,
        uri: &str,
        body: Body,
        content_type: Option<&str>,
        extra_headers: Vec<(&str, &str)>,
    ) -> (StatusCode, Response<Body>) {
        let mut req = Request::builder().method(method).uri(uri);

        if let Some(ref t) = self.token {
            req = req.header(header::AUTHORIZATION, format!("Bearer {}", t));
        }

        if let Some(ct) = content_type {
            req = req.header(header::CONTENT_TYPE, ct);
        }

        for (k, v) in extra_headers {
            req = req.header(k, v);
        }

        let req = req.body(body).unwrap();
        let resp = self.router.clone().oneshot(req).await.unwrap();
        (resp.status(), resp)
    }

    pub async fn get(&mut self, uri: &str) -> (StatusCode, Response<Body>) {
        self.request("GET", uri, Body::empty(), None).await
    }

    pub async fn post_json<T: serde::Serialize>(
        &mut self,
        uri: &str,
        json: &T,
    ) -> (StatusCode, Response<Body>) {
        let body_bytes = serde_json::to_vec(json).unwrap();
        self.request(
            "POST",
            uri,
            Body::from(body_bytes),
            Some("application/json"),
        )
        .await
    }

    pub async fn put_json<T: serde::Serialize>(
        &mut self,
        uri: &str,
        json: &T,
    ) -> (StatusCode, Response<Body>) {
        let body_bytes = serde_json::to_vec(json).unwrap();
        self.request("PUT", uri, Body::from(body_bytes), Some("application/json"))
            .await
    }

    pub async fn patch_json<T: serde::Serialize>(
        &mut self,
        uri: &str,
        json: &T,
    ) -> (StatusCode, Response<Body>) {
        let body_bytes = serde_json::to_vec(json).unwrap();
        self.request(
            "PATCH",
            uri,
            Body::from(body_bytes),
            Some("application/json"),
        )
        .await
    }

    pub async fn delete(&mut self, uri: &str) -> (StatusCode, Response<Body>) {
        self.request("DELETE", uri, Body::empty(), None).await
    }
}

pub async fn read_body_json(resp: Response<Body>) -> serde_json::Value {
    let body_bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&body_bytes).unwrap_or(serde_json::Value::Null)
}

pub async fn read_body_string(resp: Response<Body>) -> String {
    let body_bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    String::from_utf8(body_bytes.to_vec()).unwrap()
}
