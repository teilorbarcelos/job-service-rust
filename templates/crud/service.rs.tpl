use crate::{
    core::query_parser::{PaginatedResponse, ParsedFilters},
    errors::AppError,
    models::{{entity_slug}},
    modules::{{entity_slug}}::schemas::{Create{{EntityName}}Request, {{EntityName}}Response, Update{{EntityName}}Request},
};
use sea_orm::{DatabaseConnection, Set};

pub struct {{EntityName}}ModuleService;

impl {{EntityName}}ModuleService {
    pub async fn list_{{entity_slug}}s(
        filters: ParsedFilters,
        db: &DatabaseConnection,
    ) -> Result<PaginatedResponse<{{EntityName}}Response>, AppError> {
        crate::core::crud::list_records::<{{entity_slug}}::Entity, {{EntityName}}Response, _>(
            filters,
            db,
            {{EntityName}}Response::from,
        )
        .await
    }

    pub async fn get_{{entity_slug}}_by_id(
        id: &str,
        db: &DatabaseConnection,
    ) -> Result<{{EntityName}}Response, AppError> {
        let p = crate::core::crud::get_by_id::<{{entity_slug}}::Entity>(id, db).await?;
        Ok({{EntityName}}Response::from(p))
    }

    pub async fn create_{{entity_slug}}(
        payload: Create{{EntityName}}Request,
        db: &DatabaseConnection,
    ) -> Result<{{EntityName}}Response, AppError> {
        let active_item = {{entity_slug}}::ActiveModel {
{{ServiceCreateFieldsMappings}}
            ..Default::default()
        };
        let p = crate::core::crud::create_record::<{{entity_slug}}::Entity, _>(db, active_item).await?;
        Ok({{EntityName}}Response::from(p))
    }

    pub async fn update_{{entity_slug}}(
        id: &str,
        payload: Update{{EntityName}}Request,
        db: &DatabaseConnection,
    ) -> Result<{{EntityName}}Response, AppError> {
        let mut active_item = {{entity_slug}}::ActiveModel {
            id: Set(id.to_string()),
{{ServiceCreateFieldsMappings}}
            ..Default::default()
        };

        if let Some(act) = payload.active {
            active_item.active = Set(act);
        }

        let updated = crate::core::crud::update_record::<{{entity_slug}}::Entity, _>(db, active_item).await?;
        Ok({{EntityName}}Response::from(updated))
    }

    pub async fn delete_{{entity_slug}}(id: &str, db: &DatabaseConnection) -> Result<(), AppError> {
        crate::core::crud::soft_delete::<{{entity_slug}}::Entity, {{entity_slug}}::ActiveModel>(id, db).await
    }

    pub async fn toggle_{{entity_slug}}_status(
        id: &str,
        active: bool,
        db: &DatabaseConnection,
    ) -> Result<{{EntityName}}Response, AppError> {
        let updated = crate::core::crud::toggle_status::<{{entity_slug}}::Entity, {{entity_slug}}::ActiveModel>(
            id, active, db,
        )
        .await?;
        Ok({{EntityName}}Response::from(updated))
    }
}
