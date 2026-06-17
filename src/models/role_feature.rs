use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize, ToSchema)]
#[sea_orm(schema_name = "public", table_name = "RoleFeature")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id_role: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub id_feature: String,
    pub create: bool,
    pub view: bool,
    pub activate: bool,
    pub delete: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::role::Entity",
        from = "Column::IdRole",
        to = "super::role::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Role,
    #[sea_orm(
        belongs_to = "super::feature::Entity",
        from = "Column::IdFeature",
        to = "super::feature::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Feature,
}

impl Related<super::role::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Role.def()
    }
}

impl Related<super::feature::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Feature.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::Related;

    #[test]
    fn test_relations() {
        let _ = <Entity as Related<crate::models::role::Entity>>::to();
        let _ = <Entity as Related<crate::models::feature::Entity>>::to();
    }
}
