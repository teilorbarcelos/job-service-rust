use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize, ToSchema)]
#[sea_orm(schema_name = "audit", table_name = "tb_audit")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub id_user: String,
    pub user_name: String,
    pub action_type: String,
    pub execute_type: String,
    pub class: String,
    pub function: String,
    pub params: String,
    pub raw: String,
    pub table_name: String,
    pub diff_value: String,
    pub original_url: String,
    pub method: String,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
