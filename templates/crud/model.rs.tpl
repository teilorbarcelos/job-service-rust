use crate::core::query_parser::{FilterDefinition, OrderDefinition, SearchDefinition};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize, ToSchema)]
#[sea_orm(schema_name = "public", table_name = "{{EntityName}}")]
pub struct Model {
{{FieldsModel}}
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

crate::impl_crud_traits!(
    Entity,
    ActiveModel,
    Column::IsDeleted,
    Column::Active,
    "Registro não encontrado",
    |_| "Item com ID correspondente já cadastrado".to_string(),
    |_| "Item com ID correspondente já cadastrado".to_string(),
    {
        let mut filter_defs = vec![
{{ServiceListFilterDefinitions}}
            FilterDefinition::boolean("active", (Entity, Column::Active)),
        ];
        filter_defs.extend(FilterDefinition::date_range("createdAt", (Entity, Column::CreatedAt)));
        filter_defs.extend(FilterDefinition::date_range("updatedAt", (Entity, Column::UpdatedAt)));
        filter_defs
    },
    vec![
{{ServiceListSearchDefinitions}}
    ],
    vec![
{{ServiceListOrderDefinitions}}
        OrderDefinition::column("createdAt", (Entity, Column::CreatedAt)),
    ],
    Column::CreatedAt
);

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::Iterable;

    #[test]
    fn test_relations() {
        // Simple test to cover relations compilation/coverage
        let _ = Relation::iter().count();
    }
}
