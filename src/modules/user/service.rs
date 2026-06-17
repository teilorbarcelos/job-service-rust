use crate::{
    core::crud::CrudEntity,
    core::query_parser::{PaginatedResponse, ParsedFilters},
    errors::AppError,
    infra::auth::AuthService,
    infra::cache::Cache,
    models::{auth, role, user},
    modules::user::schemas::{CreateUserRequest, UpdateUserRequest, UserResponse},
};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

pub struct UserModuleService;

impl UserModuleService {
    pub async fn list_users(
        filters: ParsedFilters,
        db: &DatabaseConnection,
    ) -> Result<PaginatedResponse<UserResponse>, AppError> {
        let query = user::Entity::find()
            .left_join(role::Entity)
            .filter(user::Column::IsDeleted.ne(true));

        crate::core::crud::list_records_with_query::<user::Entity, UserResponse, _>(
            filters,
            db,
            query,
            &user::Entity::filter_definitions(),
            &user::Entity::search_definitions(),
            &user::Entity::order_definitions(),
            (user::Entity, user::Entity::default_order_column()),
            UserResponse::from,
        )
        .await
    }

    pub async fn get_user_by_id(
        id: &str,
        db: &DatabaseConnection,
    ) -> Result<UserResponse, AppError> {
        let u = crate::core::crud::get_by_id::<user::Entity>(id, db).await?;
        Ok(UserResponse::from(u))
    }

    pub async fn create_user(
        payload: CreateUserRequest,
        db: &DatabaseConnection,
    ) -> Result<UserResponse, AppError> {
        let role_exists = role::Entity::find_by_id(&payload.id_role).one(db).await?;
        if role_exists.is_none() {
            return Err(AppError::BadRequest(
                "ID de perfil (role) fornecido inválido".to_string(),
            ));
        }

        let auth_id = Uuid::new_v4().to_string();
        let hashed_pass = AuthService::hash_password(&payload.password)?;

        let active_auth = auth::ActiveModel {
            id: Set(auth_id.clone()),
            password: Set(Some(hashed_pass)),
            request_password_token: Set(None),
            request_password_expiration: Set(None),
            retries: Set(0),
            first_access: Set(true),
            active: Set(true),
            is_deleted: Set(Some(false)),
            deleted_at: Set(None),
            created_at: Set(chrono::Utc::now().into()),
            updated_at: Set(chrono::Utc::now().into()),
        };
        active_auth.insert(db).await?;

        let active_user = user::ActiveModel {
            name: Set(payload.name),
            email: Set(payload.email),
            phone: Set(payload.phone),
            document: Set(payload.document),
            id_auth: Set(Some(auth_id)),
            id_role: Set(payload.id_role),
            ..Default::default()
        };

        let u = crate::core::crud::create_record::<user::Entity, _>(db, active_user).await?;

        Ok(UserResponse::from(u))
    }

    pub async fn update_user(
        id: &str,
        payload: UpdateUserRequest,
        db: &DatabaseConnection,
        cache: &Cache,
    ) -> Result<UserResponse, AppError> {
        let role_exists = role::Entity::find_by_id(&payload.id_role).one(db).await?;
        if role_exists.is_none() {
            return Err(AppError::BadRequest(
                "ID de perfil (role) fornecido inválido".to_string(),
            ));
        }

        let mut active_user = user::ActiveModel {
            id: Set(id.to_string()),
            name: Set(payload.name),
            email: Set(payload.email),
            id_role: Set(payload.id_role),
            phone: Set(payload.phone),
            document: Set(payload.document),
            ..Default::default()
        };

        if let Some(act) = payload.active {
            active_user.active = Set(act);
        }

        let updated = crate::core::crud::update_record::<user::Entity, _>(db, active_user).await?;

        cache.invalidate_user_sessions(id).await?;

        Ok(UserResponse::from(updated))
    }

    pub async fn delete_user(
        id: &str,
        db: &DatabaseConnection,
        cache: &Cache,
    ) -> Result<(), AppError> {
        let u = crate::core::crud::get_by_id::<user::Entity>(id, db).await?;

        let now = chrono::Utc::now();

        if let Some(ref auth_id) = u.id_auth {
            if let Some(auth_rec) = auth::Entity::find_by_id(auth_id).one(db).await? {
                let mut active_auth: auth::ActiveModel = auth_rec.into();
                active_auth.active = Set(false);
                active_auth.is_deleted = Set(Some(true));
                active_auth.deleted_at = Set(Some(now.into()));
                active_auth.password = Set(None);
                active_auth.update(db).await?;
            }
        }

        let mut active_user: user::ActiveModel = u.into();
        let unique_uuid = Uuid::new_v4().to_string();

        active_user.name = Set("Deleted User".to_string());
        active_user.email = Set(format!(
            "deleted-anonymized-{}@deleted.com",
            &unique_uuid[..8]
        ));
        active_user.phone = Set(Some("00000000000".to_string()));
        active_user.document = Set(Some("00000000000".to_string()));
        active_user.active = Set(false);
        active_user.is_deleted = Set(Some(true));
        active_user.deleted_at = Set(Some(now.into()));
        active_user.avatar = Set(None);
        active_user.update(db).await?;

        cache.invalidate_user_sessions(id).await?;

        Ok(())
    }

    pub async fn toggle_user_status(
        id: &str,
        active: bool,
        db: &DatabaseConnection,
        cache: &Cache,
    ) -> Result<UserResponse, AppError> {
        let updated =
            crate::core::crud::toggle_status::<user::Entity, user::ActiveModel>(id, active, db)
                .await?;

        cache.invalidate_user_sessions(id).await?;

        Ok(UserResponse::from(updated))
    }

    pub async fn export_users_pdf(
        parsed_filters: ParsedFilters,
        pdf_service_url: &str,
        db: &DatabaseConnection,
    ) -> Result<Vec<u8>, AppError> {
        let base_query = user::Entity::find()
            .left_join(role::Entity)
            .filter(user::Column::IsDeleted.ne(true));

        let all_users = crate::core::crud::fetch_all_records_with_query::<user::Entity>(
            parsed_filters,
            db,
            base_query,
            &user::Entity::filter_definitions(),
            &user::Entity::search_definitions(),
            &user::Entity::order_definitions(),
            (user::Entity, user::Column::CreatedAt),
        )
        .await?;

        let role_ids: Vec<String> = all_users.iter().map(|u| u.id_role.clone()).collect();
        let roles = role::Entity::find()
            .filter(role::Column::Id.is_in(role_ids))
            .all(db)
            .await?;

        let local_time = chrono::Local::now().format("%d/%m/%Y %H:%M:%S").to_string();

        let users_data: Vec<serde_json::Value> = all_users
            .into_iter()
            .map(|u| {
                let role_name = roles
                    .iter()
                    .find(|r| r.id == u.id_role)
                    .map(|r| r.name.clone());
                serde_json::json!({
                    "id": u.id,
                    "name": u.name,
                    "email": u.email,
                    "phone": u.phone,
                    "roleName": role_name,
                    "active": u.active
                })
            })
            .collect();

        let pdf_data = serde_json::json!({
            "title": "Relatório de Usuários",
            "generatedAt": local_time,
            "users": users_data
        });

        let pdf_bytes =
            crate::infra::pdf::PdfProvider::generate_pdf(pdf_service_url, "user-list", pdf_data)
                .await?;

        Ok(pdf_bytes)
    }
}
