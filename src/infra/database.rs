use once_cell::sync::OnceCell;
use sea_orm::{ConnectOptions, Database, DatabaseConnection, DbErr};
use std::time::Duration;

pub static DB_CONN: OnceCell<DatabaseConnection> = OnceCell::new();

pub async fn connect(database_url: &str) -> Result<DatabaseConnection, DbErr> {
    let mut opt = ConnectOptions::new(database_url.to_string());

    opt.max_connections(50)
        .min_connections(5)
        .connect_timeout(Duration::from_secs(10))
        .acquire_timeout(Duration::from_secs(10))
        .idle_timeout(Duration::from_secs(600))
        .max_lifetime(Duration::from_secs(1800))
        .sqlx_logging(false);

    tracing::info!("Conectando ao banco de dados...");
    let db = Database::connect(opt).await?;
    let _ = DB_CONN.set(db.clone());
    tracing::info!("Conectado com sucesso!");

    Ok(db)
}
