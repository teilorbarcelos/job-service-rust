use crate::{
    config::AppConfig,
    errors::AppError,
    infra::auth::AuthService,
    infra::cache::Cache,
    models::{auth, role, role_feature, user},
    modules::auth::schemas::{
        AuthResponse, LoginRequest, PermissionInfo, RoleInfo, SimpleStatusResponse, UserInfo,
        UserMeResponse,
    },
};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

pub struct AuthModuleService;

impl AuthModuleService {
    pub async fn login(
        payload: LoginRequest,
        db: &DatabaseConnection,
        cache: &Cache,
        config: &AppConfig,
    ) -> Result<AuthResponse, AppError> {
        let user_record = user::Entity::find()
            .filter(user::Column::Email.eq(&payload.email))
            .filter(user::Column::IsDeleted.ne(true))
            .one(db)
            .await?
            .ok_or_else(|| AppError::Unauthorized("Credenciais inválidas".to_string()))?;

        if !user_record.active {
            return Err(AppError::Forbidden(
                "Usuário inativo. Login não permitido.".to_string(),
            ));
        }

        let role_record = role::Entity::find_by_id(&user_record.id_role)
            .one(db)
            .await?
            .ok_or_else(|| {
                AppError::Unauthorized("Perfil do usuário não encontrado".to_string())
            })?;

        if !role_record.active {
            return Err(AppError::Forbidden(
                "Perfil de acesso inativo. Login não permitido.".to_string(),
            ));
        }

        let auth_id = user_record
            .id_auth
            .as_ref()
            .ok_or_else(|| AppError::Unauthorized("Credenciais não configuradas".to_string()))?;

        let auth_record = auth::Entity::find_by_id(auth_id)
            .one(db)
            .await?
            .ok_or_else(|| AppError::Unauthorized("Credenciais não encontradas".to_string()))?;

        if !auth_record.active {
            return Err(AppError::Forbidden(
                "Credenciais de acesso inativas.".to_string(),
            ));
        }

        let hash = auth_record.password.as_deref().unwrap_or("");
        let matches = AuthService::verify_password(&payload.password, hash)?;
        if !matches {
            return Err(AppError::Unauthorized("Credenciais inválidas".to_string()));
        }

        let permissions_records = role_feature::Entity::find()
            .filter(role_feature::Column::IdRole.eq(&role_record.id))
            .all(db)
            .await?;

        let permissions = permissions_records
            .into_iter()
            .map(|p| PermissionInfo {
                feature: p.id_feature,
                create: p.create,
                view: p.view,
                activate: p.activate,
                delete: p.delete,
            })
            .collect::<Vec<_>>();

        let (access_token, refresh_token) = AuthService::generate_tokens(
            &user_record.id,
            &user_record.email,
            &role_record.id,
            &config.jwt_secret,
            config.jwt_expires_in,
        )?;

        cache
            .create_session(
                &user_record.id,
                &format!("access:{}", access_token),
                config.jwt_expires_in,
            )
            .await?;
        cache
            .create_session(
                &user_record.id,
                &format!("refresh:{}", refresh_token),
                7 * 24 * 60 * 60,
            )
            .await?;

        Ok(AuthResponse {
            token: access_token,
            refresh_token,
            user: UserInfo {
                id: user_record.id,
                name: user_record.name,
                email: user_record.email,
                role: RoleInfo {
                    id: role_record.id,
                    name: role_record.name,
                    permissions,
                },
            },
        })
    }

    pub async fn get_me(
        user_id: &str,
        db: &DatabaseConnection,
    ) -> Result<UserMeResponse, AppError> {
        let user_record = user::Entity::find_by_id(user_id.to_string())
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Usuário não encontrado".to_string()))?;

        let role_record = role::Entity::find_by_id(&user_record.id_role)
            .one(db)
            .await?
            .ok_or_else(|| AppError::NotFound("Perfil não encontrado".to_string()))?;

        let permissions_records = role_feature::Entity::find()
            .filter(role_feature::Column::IdRole.eq(&role_record.id))
            .all(db)
            .await?;

        let permissions = permissions_records
            .into_iter()
            .map(|p| PermissionInfo {
                feature: p.id_feature,
                create: p.create,
                view: p.view,
                activate: p.activate,
                delete: p.delete,
            })
            .collect::<Vec<_>>();

        Ok(UserMeResponse {
            user: UserInfo {
                id: user_record.id,
                name: user_record.name,
                email: user_record.email,
                role: RoleInfo {
                    id: role_record.id,
                    name: role_record.name,
                    permissions,
                },
            },
        })
    }

    pub async fn logout(user_id: &str, cache: &Cache) -> Result<SimpleStatusResponse, AppError> {
        cache.invalidate_user_sessions(user_id).await?;
        Ok(SimpleStatusResponse { status: true })
    }

    pub async fn refresh(
        refresh_token: &str,
        db: &DatabaseConnection,
        cache: &Cache,
        config: &AppConfig,
    ) -> Result<AuthResponse, AppError> {
        let claims = AuthService::verify_token(refresh_token, &config.jwt_secret)?;

        let is_valid = cache
            .validate_session(&claims.sub, &format!("refresh:{}", refresh_token))
            .await?;
        if !is_valid {
            return Err(AppError::Unauthorized(
                "Sessão revogada ou expirada".to_string(),
            ));
        }

        let user_record = user::Entity::find_by_id(claims.sub.clone())
            .one(db)
            .await?
            .ok_or_else(|| AppError::Unauthorized("Usuário não encontrado".to_string()))?;

        if !user_record.active {
            return Err(AppError::Unauthorized(
                "Conta de usuário inativa".to_string(),
            ));
        }

        let role_record = role::Entity::find_by_id(&user_record.id_role)
            .one(db)
            .await?
            .ok_or_else(|| AppError::Unauthorized("Perfil não encontrado".to_string()))?;

        if !role_record.active {
            return Err(AppError::Unauthorized("Perfil inativo".to_string()));
        }

        let permissions_records = role_feature::Entity::find()
            .filter(role_feature::Column::IdRole.eq(&role_record.id))
            .all(db)
            .await?;

        let permissions = permissions_records
            .into_iter()
            .map(|p| PermissionInfo {
                feature: p.id_feature,
                create: p.create,
                view: p.view,
                activate: p.activate,
                delete: p.delete,
            })
            .collect::<Vec<_>>();

        cache
            .delete_session(&user_record.id, &format!("refresh:{}", refresh_token))
            .await?;

        let (access_token, new_refresh_token) = AuthService::generate_tokens(
            &user_record.id,
            &user_record.email,
            &role_record.id,
            &config.jwt_secret,
            config.jwt_expires_in,
        )?;

        cache
            .create_session(
                &user_record.id,
                &format!("access:{}", access_token),
                config.jwt_expires_in,
            )
            .await?;
        cache
            .create_session(
                &user_record.id,
                &format!("refresh:{}", new_refresh_token),
                7 * 24 * 60 * 60,
            )
            .await?;

        Ok(AuthResponse {
            token: access_token,
            refresh_token: new_refresh_token,
            user: UserInfo {
                id: user_record.id,
                name: user_record.name,
                email: user_record.email,
                role: RoleInfo {
                    id: role_record.id,
                    name: role_record.name,
                    permissions,
                },
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::auth::schemas::LoginRequest;
    use sea_orm::{ActiveModelTrait, ConnectionTrait, DatabaseBackend, Set, Statement};

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
    async fn test_login_role_not_found() {
        if let Some(db) = get_real_db().await {
            let config = AppConfig::load();
            let cache = Cache::new(&config.redis_url);

            let auth_id = format!("a-{}", uuid::Uuid::new_v4());
            let password_hash = bcrypt::hash("password123", bcrypt::DEFAULT_COST).unwrap();
            let temp_auth = crate::models::auth::ActiveModel {
                id: Set(auth_id.clone()),
                password: Set(Some(password_hash)),
                active: Set(true),
                created_at: Set(chrono::Utc::now().into()),
                updated_at: Set(chrono::Utc::now().into()),
                ..Default::default()
            };
            temp_auth.insert(&db).await.unwrap();

            let _ = db
                .execute(Statement::from_string(
                    DatabaseBackend::Postgres,
                    "ALTER TABLE \"User\" DISABLE TRIGGER ALL".to_string(),
                ))
                .await;

            let user_id = format!("u-{}", uuid::Uuid::new_v4());
            let email = format!("{}@test.com", user_id);
            let temp_user = crate::models::user::ActiveModel {
                id: Set(user_id.clone()),
                name: Set("Temp User".to_string()),
                email: Set(email.clone()),
                id_role: Set("non-existent-role".to_string()),
                id_auth: Set(Some(auth_id.clone())),
                active: Set(true),
                is_deleted: Set(Some(false)),
                deleted_at: Set(None),
                created_at: Set(chrono::Utc::now().into()),
                updated_at: Set(chrono::Utc::now().into()),
                ..Default::default()
            };
            temp_user.insert(&db).await.unwrap();

            let payload = LoginRequest {
                email: email.clone(),
                password: "password123".to_string(),
            };

            let res = AuthModuleService::login(payload, &db, &cache, &config).await;
            assert!(res.is_err());
            assert_eq!(
                res.unwrap_err().message(),
                "Perfil do usuário não encontrado"
            );

            let _ = crate::models::user::Entity::delete_by_id(&user_id)
                .exec(&db)
                .await;
            let _ = crate::models::auth::Entity::delete_by_id(&auth_id)
                .exec(&db)
                .await;
            let _ = db
                .execute(Statement::from_string(
                    DatabaseBackend::Postgres,
                    "ALTER TABLE \"User\" ENABLE TRIGGER ALL".to_string(),
                ))
                .await;
        }
    }
}
