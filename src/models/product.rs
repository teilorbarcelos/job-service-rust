use crate::core::query_parser::{FilterDefinition, OrderDefinition, SearchDefinition};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize, ToSchema)]
#[sea_orm(schema_name = "public", table_name = "Product")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub name: String,
    pub sku: String,
    pub category: String,
    pub price: Decimal,
    pub stock: i32,
    pub description: String,
    pub active: bool,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    pub is_deleted: Option<bool>,
    pub deleted_at: Option<DateTimeWithTimeZone>,
    pub id_user: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::IdUser",
        to = "super::user::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    User,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

crate::impl_crud_traits!(
    Entity,
    ActiveModel,
    Column::IsDeleted,
    Column::Active,
    "Produto não encontrado",
    |_| "SKU já cadastrado no sistema".to_string(),
    |_| "SKU já está sendo utilizado por outro produto".to_string(),
    {
        let mut filter_defs = vec![
            FilterDefinition::contains("name", Column::Name),
            FilterDefinition::equals("sku", Column::Sku),
            FilterDefinition::equals("category", Column::Category),
            FilterDefinition::boolean("active", Column::Active),
        ];
        filter_defs.extend(FilterDefinition::date_range("createdAt", Column::CreatedAt));
        filter_defs.extend(FilterDefinition::date_range("updatedAt", Column::UpdatedAt));
        filter_defs
    },
    vec![
        SearchDefinition::contains("name", Column::Name),
        SearchDefinition::contains("sku", Column::Sku),
        SearchDefinition::contains("category", Column::Category),
    ],
    vec![
        OrderDefinition::case_insensitive("name", Column::Name),
        OrderDefinition::column("sku", Column::Sku),
        OrderDefinition::case_insensitive("category", Column::Category),
        OrderDefinition::column("createdAt", Column::CreatedAt),
    ],
    Column::CreatedAt
);

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::Related;

    #[test]
    fn test_relations() {
        let _ = <Entity as Related<crate::models::user::Entity>>::to();
    }
}
