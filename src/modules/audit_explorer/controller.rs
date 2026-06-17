use crate::{
    errors::AppError,
    middleware::auth::CurrentUser,
    models::{audit, error_log},
};
use axum::{
    extract::{Query, State},
    response::Html,
    Extension, Json,
};
use sea_orm::{
    sea_query::{Expr, Func},
    Condition, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct ExplorerQuery {
    pub page: Option<u64>,
    pub size: Option<u64>,
    pub search: Option<String>,
}

#[derive(Serialize)]
pub struct PaginatedResponse<T> {
    pub items: Vec<T>,
    pub total: u64,
}

pub async fn logs_html_handler() -> Html<&'static str> {
    Html(super::view::get_audit_explorer_view())
}

pub async fn get_audit_logs_handler(
    State((db, _, _)): State<(
        DatabaseConnection,
        crate::infra::cache::Cache,
        crate::config::AppConfig,
    )>,
    Extension(current_user): Extension<CurrentUser>,
    Query(query): Query<ExplorerQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    if current_user.role != "administrator" {
        return Err(AppError::Forbidden(
            "Apenas administradores podem acessar este recurso".to_string(),
        ));
    }

    let page = query.page.unwrap_or(0);
    let size = query.size.unwrap_or(15);
    let offset = page * size;

    let mut select = audit::Entity::find();

    if let Some(ref search) = query.search {
        if !search.trim().is_empty() {
            let search_pattern = format!("%{}%", search.to_lowercase());
            select = select.filter(
                Condition::any()
                    .add(
                        Expr::expr(Func::lower(Expr::col(audit::Column::UserName)))
                            .like(search_pattern.clone()),
                    )
                    .add(
                        Expr::expr(Func::lower(Expr::col(audit::Column::TableName)))
                            .like(search_pattern.clone()),
                    )
                    .add(
                        Expr::expr(Func::lower(Expr::col(audit::Column::ActionType)))
                            .like(search_pattern.clone()),
                    )
                    .add(
                        Expr::expr(Func::lower(Expr::col(audit::Column::OriginalUrl)))
                            .like(search_pattern.clone()),
                    )
                    .add(
                        Expr::expr(Func::lower(Expr::col(audit::Column::Method)))
                            .like(search_pattern.clone()),
                    )
                    .add(
                        Expr::expr(Func::lower(Expr::col(audit::Column::Class)))
                            .like(search_pattern.clone()),
                    )
                    .add(
                        Expr::expr(Func::lower(Expr::col(audit::Column::Function)))
                            .like(search_pattern.clone()),
                    )
                    .add(
                        Expr::expr(Func::lower(Expr::col(audit::Column::Params)))
                            .like(search_pattern.clone()),
                    )
                    .add(
                        Expr::expr(Func::lower(Expr::col(audit::Column::Raw)))
                            .like(search_pattern.clone()),
                    )
                    .add(
                        Expr::expr(Func::lower(Expr::col(audit::Column::DiffValue)))
                            .like(search_pattern.clone()),
                    ),
            );
        }
    }

    let total = select.clone().paginate(&db, 1).num_items().await?;

    let records = select
        .order_by_desc(audit::Column::CreatedAt)
        .limit(size)
        .offset(offset)
        .all(&db)
        .await?;

    let items: Vec<serde_json::Value> = records
        .into_iter()
        .map(|a| {
            serde_json::json!({
                "id": a.id,
                "id_user": a.id_user,
                "user_name": a.user_name,
                "action_type": a.action_type,
                "execute_type": a.execute_type,
                "class": a.class,
                "function": a.function,
                "params": a.params,
                "raw": a.raw,
                "table_name": a.table_name,
                "diff_value": a.diff_value,
                "original_url": a.original_url,
                "base_url": a.original_url,
                "method": a.method,
                "created_at": a.created_at.to_rfc3339(),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "items": items,
        "total": total,
    })))
}

pub async fn get_error_logs_handler(
    State((db, _, _)): State<(
        DatabaseConnection,
        crate::infra::cache::Cache,
        crate::config::AppConfig,
    )>,
    Extension(current_user): Extension<CurrentUser>,
    Query(query): Query<ExplorerQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    if current_user.role != "administrator" {
        return Err(AppError::Forbidden(
            "Apenas administradores podem acessar este recurso".to_string(),
        ));
    }

    let page = query.page.unwrap_or(0);
    let size = query.size.unwrap_or(15);
    let offset = page * size;

    let mut select = error_log::Entity::find();

    if let Some(ref search) = query.search {
        if !search.trim().is_empty() {
            let search_pattern = format!("%{}%", search.to_lowercase());
            select = select.filter(
                Condition::any()
                    .add(
                        Expr::expr(Func::lower(Expr::col(error_log::Column::IdUser)))
                            .like(search_pattern.clone()),
                    )
                    .add(
                        Expr::expr(Func::lower(Expr::col(error_log::Column::Source)))
                            .like(search_pattern.clone()),
                    )
                    .add(
                        Expr::expr(Func::lower(Expr::col(error_log::Column::ErrorMessage)))
                            .like(search_pattern.clone()),
                    )
                    .add(
                        Expr::expr(Func::lower(Expr::col(error_log::Column::ErrorData)))
                            .like(search_pattern.clone()),
                    ),
            );
        }
    }

    let total = select.clone().paginate(&db, 1).num_items().await?;

    let records = select
        .order_by_desc(error_log::Column::CreatedAt)
        .limit(size)
        .offset(offset)
        .all(&db)
        .await?;

    let items: Vec<serde_json::Value> = records
        .into_iter()
        .map(|e| {
            serde_json::json!({
                "id": e.id,
                "id_user": e.id_user,
                "source": e.source,
                "error_message": e.error_message,
                "error_data": e.error_data,
                "created_at": e.created_at.to_rfc3339(),
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "items": items,
        "total": total,
    })))
}
