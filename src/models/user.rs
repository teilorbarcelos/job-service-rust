use crate::core::query_parser::{FilterDefinition, OrderDefinition, SearchDefinition};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize, ToSchema)]
#[sea_orm(schema_name = "public", table_name = "User")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub name: String,
    pub phone: Option<String>,
    pub email: String,
    pub cognito_id: Option<String>,
    pub active: bool,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    pub document: Option<String>,
    pub is_deleted: Option<bool>,
    pub deleted_at: Option<DateTimeWithTimeZone>,
    pub avatar: Option<String>,
    pub id_auth: Option<String>,
    pub id_role: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::auth::Entity",
        from = "Column::IdAuth",
        to = "super::auth::Column::Id",
        on_update = "Cascade",
        on_delete = "SetNull"
    )]
    Auth,
    #[sea_orm(
        belongs_to = "super::role::Entity",
        from = "Column::IdRole",
        to = "super::role::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Role,
}

impl Related<super::auth::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Auth.def()
    }
}

impl Related<super::role::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Role.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

crate::impl_crud_traits!(
    Entity,
    ActiveModel,
    Column::IsDeleted,
    Column::Active,
    "Usuário não encontrado",
    |_| "E-mail já cadastrado no sistema".to_string(),
    |_| "E-mail já está sendo utilizado por outro usuário".to_string(),
    {
        let mut filter_defs = vec![
            FilterDefinition::contains("name", (Entity, Column::Name)),
            FilterDefinition::contains("email", (Entity, Column::Email)),
            FilterDefinition::boolean("active", (Entity, Column::Active)),
            FilterDefinition::contains(
                "Role.name",
                (super::role::Entity, super::role::Column::Name),
            ),
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
        SearchDefinition::contains("email", (Entity, Column::Email)),
        SearchDefinition::contains(
            "Role.name",
            (super::role::Entity, super::role::Column::Name)
        ),
    ],
    vec![
        OrderDefinition::case_insensitive("name", (Entity, Column::Name)),
        OrderDefinition::case_insensitive("email", (Entity, Column::Email)),
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
        let _ = <Entity as Related<crate::models::auth::Entity>>::to();
        let _ = <Entity as Related<crate::models::role::Entity>>::to();
    }
}
