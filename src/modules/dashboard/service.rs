use crate::{
    errors::AppError,
    models::{product, user},
    modules::dashboard::schemas::{
        DashboardStatsResponseDto, TimeSeriesStatDto, UserProductStatDto,
    },
};
use chrono::{DateTime, Utc};
use sea_orm::{
    sea_query::Expr, ColumnTrait, DatabaseConnection, EntityTrait, FromQueryResult, QueryFilter,
    QueryOrder, QuerySelect, RelationTrait,
};

#[derive(FromQueryResult)]
struct TimeSeriesStat {
    pub date: String,
    pub count: i64,
}

#[derive(FromQueryResult)]
struct UserProductStat {
    pub user_id: Option<String>,
    pub user_name: Option<String>,
    pub count: i64,
}

pub struct DashboardModuleService;

impl DashboardModuleService {
    pub async fn get_stats(
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        db: &DatabaseConnection,
    ) -> Result<DashboardStatsResponseDto, AppError> {
        let user_stats_raw = user::Entity::find()
            .select_only()
            .column_as(
                Expr::cust("TO_CHAR(created_at AT TIME ZONE 'America/Sao_Paulo', 'YYYY-MM-DD')"),
                "date",
            )
            .column_as(Expr::cust("COUNT(*)"), "count")
            .filter(user::Column::CreatedAt.gte(start))
            .filter(user::Column::CreatedAt.lte(end))
            .filter(user::Column::IsDeleted.ne(true))
            .group_by(Expr::cust(
                "TO_CHAR(created_at AT TIME ZONE 'America/Sao_Paulo', 'YYYY-MM-DD')",
            ))
            .order_by_asc(Expr::cust(
                "TO_CHAR(created_at AT TIME ZONE 'America/Sao_Paulo', 'YYYY-MM-DD')",
            ))
            .into_model::<TimeSeriesStat>()
            .all(db)
            .await?;

        let product_stats_raw = product::Entity::find()
            .select_only()
            .column_as(
                Expr::cust("TO_CHAR(created_at AT TIME ZONE 'America/Sao_Paulo', 'YYYY-MM-DD')"),
                "date",
            )
            .column_as(Expr::cust("COUNT(*)"), "count")
            .filter(product::Column::CreatedAt.gte(start))
            .filter(product::Column::CreatedAt.lte(end))
            .filter(product::Column::IsDeleted.ne(true))
            .group_by(Expr::cust(
                "TO_CHAR(created_at AT TIME ZONE 'America/Sao_Paulo', 'YYYY-MM-DD')",
            ))
            .order_by_asc(Expr::cust(
                "TO_CHAR(created_at AT TIME ZONE 'America/Sao_Paulo', 'YYYY-MM-DD')",
            ))
            .into_model::<TimeSeriesStat>()
            .all(db)
            .await?;

        let products_per_user_raw = product::Entity::find()
            .select_only()
            .column_as(product::Column::IdUser, "user_id")
            .column_as(user::Column::Name, "user_name")
            .column_as(Expr::cust("COUNT(*)"), "count")
            .join(sea_orm::JoinType::LeftJoin, product::Relation::User.def())
            .filter(product::Column::CreatedAt.gte(start))
            .filter(product::Column::CreatedAt.lte(end))
            .filter(product::Column::IsDeleted.ne(true))
            .group_by(product::Column::IdUser)
            .group_by(user::Column::Name)
            .order_by_desc(Expr::cust("COUNT(*)"))
            .into_model::<UserProductStat>()
            .all(db)
            .await?;

        let user_creation_stats = user_stats_raw
            .into_iter()
            .map(|item| TimeSeriesStatDto {
                date: item.date,
                count: item.count as i32,
            })
            .collect();

        let product_creation_stats = product_stats_raw
            .into_iter()
            .map(|item| TimeSeriesStatDto {
                date: item.date,
                count: item.count as i32,
            })
            .collect();

        let products_per_user = products_per_user_raw
            .into_iter()
            .map(|item| UserProductStatDto {
                user_id: item.user_id,
                user_name: item.user_name.unwrap_or_else(|| "Anonymous".to_string()),
                count: item.count as i32,
            })
            .collect();

        Ok(DashboardStatsResponseDto {
            user_creation_stats,
            product_creation_stats,
            products_per_user,
        })
    }
}
