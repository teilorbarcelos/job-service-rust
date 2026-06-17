use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared(
            r#"
            CREATE SCHEMA IF NOT EXISTS audit;

            CREATE TABLE IF NOT EXISTS audit.tb_audit (
                id VARCHAR(40) PRIMARY KEY,
                id_user VARCHAR(255),
                user_name VARCHAR(255),
                action_type VARCHAR(255),
                execute_type VARCHAR(255),
                class VARCHAR(255),
                function VARCHAR(255),
                params TEXT,
                raw TEXT,
                table_name VARCHAR(255),
                diff_value TEXT,
                error TEXT,
                host TEXT,
                ip TEXT,
                base_url TEXT,
                method TEXT,
                hostname TEXT,
                original_url TEXT,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL
            );

            CREATE TABLE IF NOT EXISTS audit.tb_error_log (
                id VARCHAR(40) PRIMARY KEY,
                id_user VARCHAR(255),
                source TEXT,
                error_message TEXT,
                error_data TEXT,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL
            );
        "#,
        )
        .await?;

        db.execute_unprepared(
            r#"
            CREATE TABLE IF NOT EXISTS public."Role" (
                id VARCHAR(40) PRIMARY KEY,
                name VARCHAR(255) NOT NULL,
                description VARCHAR(255) NOT NULL,
                active BOOLEAN DEFAULT TRUE NOT NULL,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL,
                updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL,
                is_deleted BOOLEAN DEFAULT FALSE,
                deleted_at TIMESTAMP WITH TIME ZONE
            );

            CREATE TABLE IF NOT EXISTS public."Feature" (
                id VARCHAR(40) PRIMARY KEY,
                name VARCHAR(255) NOT NULL,
                description TEXT NOT NULL,
                active BOOLEAN DEFAULT TRUE NOT NULL,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL,
                updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL
            );

            CREATE TABLE IF NOT EXISTS public."Auth" (
                id VARCHAR(40) PRIMARY KEY,
                password VARCHAR(255),
                request_password_token VARCHAR(255),
                request_password_expiration TIMESTAMP WITH TIME ZONE,
                retries INTEGER DEFAULT 0 NOT NULL,
                first_access BOOLEAN DEFAULT TRUE NOT NULL,
                active BOOLEAN DEFAULT TRUE NOT NULL,
                is_deleted BOOLEAN DEFAULT FALSE,
                deleted_at TIMESTAMP WITH TIME ZONE,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL,
                updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL
            );

            CREATE TABLE IF NOT EXISTS public."User" (
                id VARCHAR(40) PRIMARY KEY,
                name VARCHAR(255) NOT NULL,
                phone VARCHAR(15),
                email VARCHAR(255) UNIQUE NOT NULL,
                cognito_id VARCHAR(255) UNIQUE,
                active BOOLEAN DEFAULT TRUE NOT NULL,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL,
                updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL,
                document VARCHAR(20),
                is_deleted BOOLEAN DEFAULT FALSE,
                deleted_at TIMESTAMP WITH TIME ZONE,
                avatar VARCHAR(255),
                id_auth VARCHAR(40) UNIQUE REFERENCES public."Auth"(id) ON DELETE SET NULL,
                id_role VARCHAR(40) NOT NULL REFERENCES public."Role"(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS public."RoleFeature" (
                id_role VARCHAR(40) NOT NULL REFERENCES public."Role"(id) ON DELETE CASCADE,
                id_feature VARCHAR(40) NOT NULL REFERENCES public."Feature"(id) ON DELETE CASCADE,
                "create" BOOLEAN DEFAULT FALSE NOT NULL,
                "view" BOOLEAN DEFAULT FALSE NOT NULL,
                "activate" BOOLEAN DEFAULT FALSE NOT NULL,
                "delete" BOOLEAN DEFAULT FALSE NOT NULL,
                PRIMARY KEY (id_feature, id_role)
            );

            CREATE TABLE IF NOT EXISTS public."Product" (
                id VARCHAR(40) PRIMARY KEY,
                name VARCHAR(255) NOT NULL,
                sku VARCHAR(100) UNIQUE NOT NULL,
                category VARCHAR(255) NOT NULL,
                price NUMERIC(10, 2) NOT NULL,
                stock INTEGER DEFAULT 0 NOT NULL,
                description TEXT NOT NULL,
                active BOOLEAN DEFAULT TRUE NOT NULL,
                created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL,
                updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL,
                is_deleted BOOLEAN DEFAULT FALSE,
                deleted_at TIMESTAMP WITH TIME ZONE,
                id_user VARCHAR(40) REFERENCES public."User"(id) ON DELETE SET NULL
            );
        "#,
        )
        .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();

        db.execute_unprepared(
            r#"
            DROP TABLE IF EXISTS public."Product" CASCADE;
            DROP TABLE IF EXISTS public."RoleFeature" CASCADE;
            DROP TABLE IF EXISTS public."User" CASCADE;
            DROP TABLE IF EXISTS public."Auth" CASCADE;
            DROP TABLE IF EXISTS public."Feature" CASCADE;
            DROP TABLE IF EXISTS public."Role" CASCADE;
            DROP TABLE IF EXISTS audit.tb_error_log CASCADE;
            DROP TABLE IF EXISTS audit.tb_audit CASCADE;
            DROP SCHEMA IF EXISTS audit CASCADE;
        "#,
        )
        .await?;

        Ok(())
    }
}
