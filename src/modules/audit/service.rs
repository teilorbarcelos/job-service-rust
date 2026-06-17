use crate::{
    core::query_parser::{PaginatedResponse, ParsedFilters},
    errors::AppError,
    models::audit,
    modules::audit::schemas::AuditLogResponse,
};
use sea_orm::{DatabaseConnection, EntityTrait, PaginatorTrait, QuerySelect};

pub struct AuditModuleService;

impl AuditModuleService {
    pub async fn list_audit_logs(
        filters: ParsedFilters,
        db: &DatabaseConnection,
    ) -> Result<PaginatedResponse<AuditLogResponse>, AppError> {
        use crate::core::query_parser::{FilterDefinition, OrderDefinition, SearchDefinition};

        let filter_defs = FilterDefinition::date_range("createdAt", audit::Column::CreatedAt);

        let search_defs = vec![SearchDefinition::contains(
            "username",
            audit::Column::UserName,
        )];

        let order_defs = vec![OrderDefinition::column(
            "createdAt",
            audit::Column::CreatedAt,
        )];

        let mut query = audit::Entity::find();

        query = filters.apply_search(query, &search_defs);
        query = filters.apply_filters(query, &filter_defs);

        let total = query.clone().paginate(db, 1).num_items().await?;

        query = filters.apply_order(query, &order_defs, audit::Column::CreatedAt);

        let offset = filters.page * filters.size;
        let records = query.limit(filters.size).offset(offset).all(db).await?;

        let items = records
            .into_iter()
            .map(|a| AuditLogResponse {
                id: a.id,
                id_user: a.id_user,
                user_name: a.user_name,
                action_type: a.action_type,
                execute_type: a.execute_type,
                class: a.class,
                function: a.function,
                params: a.params,
                raw: a.raw,
                table_name: a.table_name,
                diff_value: a.diff_value,
                original_url: a.original_url,
                method: a.method,
                created_at: a.created_at.to_rfc3339(),
            })
            .collect();

        Ok(PaginatedResponse {
            items,
            total,
            page: filters.page,
            size: filters.size,
        })
    }
}
