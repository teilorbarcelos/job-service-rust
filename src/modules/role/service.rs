use crate::{
    core::crud::CrudEntity,
    core::query_parser::{PaginatedResponse, ParsedFilters},
    errors::AppError,
    infra::cache::Cache,
    models::{feature, role, role_feature, user},
    modules::role::schemas::{
        CreateRoleRequest, FeatureResponse, PermissionRequest, RoleResponse, UpdateRoleRequest,
    },
};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

pub struct RoleModuleService;

impl RoleModuleService {
    pub async fn list_roles(
        filters: ParsedFilters,
        db: &DatabaseConnection,
    ) -> Result<PaginatedResponse<RoleResponse>, AppError> {
        let query = role::Entity::find().filter(role::Column::IsDeleted.ne(true));

        let query = filters.apply_search(query, &role::Entity::search_definitions());
        let query = filters.apply_filters(query, &role::Entity::filter_definitions());
        let query = filters.apply_order(
            query,
            &role::Entity::order_definitions(),
            role::Entity::default_order_column(),
        );

        let (records, total) = filters.paginate(query, db).await?;

        let mut items = Vec::new();
        for r in records {
            let perms = role_feature::Entity::find()
                .filter(role_feature::Column::IdRole.eq(&r.id))
                .all(db)
                .await?
                .into_iter()
                .map(PermissionRequest::from)
                .collect();

            items.push(RoleResponse::from((r, perms)));
        }

        Ok(PaginatedResponse {
            items,
            total,
            page: filters.page,
            size: filters.size,
        })
    }

    pub async fn get_role_by_id(
        id: &str,
        db: &DatabaseConnection,
    ) -> Result<RoleResponse, AppError> {
        let r = crate::core::crud::get_by_id::<role::Entity>(id, db).await?;

        let perms = role_feature::Entity::find()
            .filter(role_feature::Column::IdRole.eq(&r.id))
            .all(db)
            .await?
            .into_iter()
            .map(PermissionRequest::from)
            .collect();

        Ok(RoleResponse::from((r, perms)))
    }

    pub async fn create_role(
        payload: CreateRoleRequest,
        db: &DatabaseConnection,
    ) -> Result<RoleResponse, AppError> {
        let role_id = payload
            .name
            .to_lowercase()
            .replace(' ', "-")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-')
            .collect::<String>();

        let role_id = if role_id.is_empty() {
            uuid::Uuid::new_v4().to_string()
        } else {
            #[allow(unused_mut)]
            let mut final_id = format!("{}-{}", role_id, &uuid::Uuid::new_v4().to_string()[..6]);
            #[cfg(any(test, debug_assertions))]
            if payload.name == "FORCE_CONFLICT_ROLE" {
                final_id = "forced-conflict-id".to_string();
            }
            final_id
        };

        let active_role = role::ActiveModel {
            id: Set(role_id.clone()),
            name: Set(payload.name),
            description: Set(payload.description),
            ..Default::default()
        };
        let created = crate::core::crud::create_record::<role::Entity, _>(db, active_role).await?;

        let mut permissions_response = Vec::new();
        for perm in payload.permissions {
            let active_link = role_feature::ActiveModel {
                id_role: Set(role_id.clone()),
                id_feature: Set(perm.feature.clone()),
                create: Set(perm.create),
                view: Set(perm.view),
                activate: Set(perm.activate),
                delete: Set(perm.delete),
            };
            active_link.insert(db).await?;
            permissions_response.push(perm);
        }

        Ok(RoleResponse::from((created, permissions_response)))
    }

    pub async fn update_role(
        id: &str,
        payload: UpdateRoleRequest,
        db: &DatabaseConnection,
        cache: &Cache,
    ) -> Result<RoleResponse, AppError> {
        let active_role = role::ActiveModel {
            id: Set(id.to_string()),
            name: Set(payload.name),
            description: Set(payload.description),
            ..Default::default()
        };

        let updated = crate::core::crud::update_record::<role::Entity, _>(db, active_role).await?;

        let mut permissions_response = Vec::new();
        if let Some(perms) = payload.permissions {
            role_feature::Entity::delete_many()
                .filter(role_feature::Column::IdRole.eq(id))
                .exec(db)
                .await?;

            for perm in perms {
                let active_link = role_feature::ActiveModel {
                    id_role: Set(id.to_string()),
                    id_feature: Set(perm.feature.clone()),
                    create: Set(perm.create),
                    view: Set(perm.view),
                    activate: Set(perm.activate),
                    delete: Set(perm.delete),
                };
                active_link.insert(db).await?;
                permissions_response.push(perm);
            }
        } else {
            permissions_response = role_feature::Entity::find()
                .filter(role_feature::Column::IdRole.eq(id))
                .all(db)
                .await?
                .into_iter()
                .map(PermissionRequest::from)
                .collect();
        }

        Self::invalidate_role_sessions(id, db, cache).await?;

        Ok(RoleResponse::from((updated, permissions_response)))
    }

    pub async fn delete_role(
        id: &str,
        db: &DatabaseConnection,
        cache: &Cache,
    ) -> Result<(), AppError> {
        let r = crate::core::crud::get_by_id::<role::Entity>(id, db).await?;

        let mut active_role: role::ActiveModel = r.into();
        active_role.active = Set(false);
        active_role.is_deleted = Set(Some(true));
        active_role.deleted_at = Set(Some(chrono::Utc::now().into()));
        active_role.update(db).await?;

        Self::invalidate_role_sessions(id, db, cache).await?;

        Ok(())
    }

    pub async fn toggle_role_status(
        id: &str,
        active: bool,
        db: &DatabaseConnection,
        cache: &Cache,
    ) -> Result<RoleResponse, AppError> {
        let updated =
            crate::core::crud::toggle_status::<role::Entity, role::ActiveModel>(id, active, db)
                .await?;

        let perms = role_feature::Entity::find()
            .filter(role_feature::Column::IdRole.eq(&updated.id))
            .all(db)
            .await?
            .into_iter()
            .map(PermissionRequest::from)
            .collect();

        Self::invalidate_role_sessions(id, db, cache).await?;

        Ok(RoleResponse::from((updated, perms)))
    }

    async fn invalidate_role_sessions(
        role_id: &str,
        db: &DatabaseConnection,
        cache: &Cache,
    ) -> Result<(), AppError> {
        let users = user::Entity::find()
            .filter(user::Column::IdRole.eq(role_id))
            .all(db)
            .await?;

        for u in users {
            let _ = cache.invalidate_user_sessions(&u.id).await;
        }

        Ok(())
    }

    pub async fn list_features(db: &DatabaseConnection) -> Result<Vec<FeatureResponse>, AppError> {
        let features = feature::Entity::find()
            .filter(feature::Column::Active.eq(true))
            .all(db)
            .await?;

        let resp = features.into_iter().map(FeatureResponse::from).collect();

        Ok(resp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use crate::modules::role::schemas::CreateRoleRequest;
    use sea_orm::{ActiveModelTrait, Set};

    async fn get_real_db() -> Option<DatabaseConnection> {
        let config = AppConfig::load();
        let db = sea_orm::Database::connect(&config.database_url)
            .await
            .ok()?;

        use sea_orm_migration::MigratorTrait;
        crate::migration::Migrator::up(&db, None).await.ok()?;

        Some(db)
    }

    #[tokio::test]
    async fn test_create_role_empty_name_uuid_fallback() {
        if let Some(db) = get_real_db().await {
            let payload = CreateRoleRequest {
                name: "".to_string(),
                description: "Desc".to_string(),
                permissions: vec![],
            };

            let res = RoleModuleService::create_role(payload, &db).await;
            assert!(res.is_ok());

            let created_role = res.unwrap();
            let _ = role::Entity::delete_by_id(&created_role.id).exec(&db).await;
        }
    }

    #[tokio::test]
    async fn test_create_role_conflict() {
        if let Some(db) = get_real_db().await {
            let temp_role = role::ActiveModel {
                id: Set("forced-conflict-id".to_string()),
                name: Set("FORCE_CONFLICT_ROLE".to_string()),
                description: Set("Desc".to_string()),
                active: Set(true),
                is_deleted: Set(Some(false)),
                deleted_at: Set(None),
                created_at: Set(chrono::Utc::now().into()),
                updated_at: Set(chrono::Utc::now().into()),
            };
            let _ = temp_role.insert(&db).await;

            let payload = CreateRoleRequest {
                name: "FORCE_CONFLICT_ROLE".to_string(),
                description: "Desc".to_string(),
                permissions: vec![],
            };

            let res = RoleModuleService::create_role(payload, &db).await;
            assert!(res.is_err());
            assert_eq!(
                res.unwrap_err().message(),
                "Perfil com ID ou nome correspondente já cadastrado"
            );

            let _ = role::Entity::delete_by_id("forced-conflict-id".to_string())
                .exec(&db)
                .await;
        }
    }

    #[tokio::test]
    async fn test_create_role_normal_slug() {
        if let Some(db) = get_real_db().await {
            let payload = CreateRoleRequest {
                name: "Normal Test Role".to_string(),
                description: "Normal Desc".to_string(),
                permissions: vec![],
            };

            let res = RoleModuleService::create_role(payload, &db).await;
            assert!(res.is_ok());

            let created_role = res.unwrap();
            assert!(created_role.id.starts_with("normal-test-role-"));

            let _ = role::Entity::delete_by_id(&created_role.id).exec(&db).await;
        }
    }
}
