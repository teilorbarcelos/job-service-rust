use crate::core::query_parser::{FilterDefinition, OrderDefinition, SearchDefinition};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize, ToSchema)]
#[sea_orm(schema_name = "public", table_name = "Role")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub name: String,
    pub description: String,
    pub active: bool,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    pub is_deleted: Option<bool>,
    pub deleted_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::user::Entity")]
    User,
    #[sea_orm(has_many = "super::role_feature::Entity")]
    RoleFeature,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<super::role_feature::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::RoleFeature.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

crate::impl_crud_traits!(
    Entity,
    ActiveModel,
    Column::IsDeleted,
    Column::Active,
    "Perfil não encontrado",
    |_| "Perfil com ID ou nome correspondente já cadastrado".to_string(),
    |_| "Perfil com ID ou nome correspondente já cadastrado".to_string(),
    {
        let mut filter_defs = vec![
            FilterDefinition::contains("name", (Entity, Column::Name)),
            FilterDefinition::contains("description", (Entity, Column::Description)),
            FilterDefinition::boolean("active", (Entity, Column::Active)),
        ];
        filter_defs.extend(FilterDefinition::date_range(
            "createdAt",
            (Entity, Column::CreatedAt),
        ));
        filter_defs.extend(FilterDefinition::date_range(
            "updatedAt",
            (Entity, Column::UpdatedAt),
        ));
        filter_defs
    },
    vec![
        SearchDefinition::contains("name", (Entity, Column::Name)),
        SearchDefinition::contains("description", (Entity, Column::Description)),
    ],
    vec![
        OrderDefinition::case_insensitive("name", (Entity, Column::Name)),
        OrderDefinition::case_insensitive("description", (Entity, Column::Description)),
        OrderDefinition::column("createdAt", (Entity, Column::CreatedAt)),
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
        let _ = <Entity as Related<crate::models::role_feature::Entity>>::to();
    }

    #[test]
    fn test_role_update_conflict_message() {
        use crate::core::crud::CrudEntity;
        let msg = Entity::update_conflict_error_message("test");
        assert_eq!(msg, "Perfil com ID ou nome correspondente já cadastrado");
    }
}
