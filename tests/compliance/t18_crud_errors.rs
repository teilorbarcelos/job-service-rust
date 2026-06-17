use crate::common::TestContext;
use backend_rust::{
    core::crud::{create_record, soft_delete, toggle_status, update_record, CrudEntity},
    errors::AppError,
};
use sea_orm::{
    entity::prelude::*, ActiveModelBehavior, ActiveModelTrait, ConnectionTrait, EntityTrait,
    IntoActiveModel, Set, Statement,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(schema_name = "public", table_name = "test_crud_temp")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub name: String,
    pub active: bool,
    pub is_deleted: Option<bool>,
    pub deleted_at: Option<DateTimeWithTimeZone>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

backend_rust::impl_crud_traits!(
    Entity,
    ActiveModel,
    Column::IsDeleted,
    Column::Active,
    "Record not found",
    |_| "Conflict created".to_string(),
    |_| "Conflict updated".to_string(),
    vec![],
    vec![],
    vec![],
    Column::Id
);

pub async fn run(ctx: &TestContext) {
    println!("=== Running CRUD Errors Coverage Tests ===");
    let db = &ctx.db;

    db.execute(Statement::from_string(
        db.get_database_backend(),
        "DROP TABLE IF EXISTS public.test_crud_temp CASCADE;".to_string(),
    ))
    .await
    .unwrap();

    db.execute(Statement::from_string(
        db.get_database_backend(),
        r#"
        CREATE TABLE public.test_crud_temp (
            id VARCHAR(40) PRIMARY KEY,
            name VARCHAR(255) NOT NULL,
            active BOOLEAN DEFAULT TRUE NOT NULL,
            is_deleted BOOLEAN DEFAULT FALSE,
            deleted_at TIMESTAMP WITH TIME ZONE,
            created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL,
            updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP NOT NULL,
            CONSTRAINT chk_name CHECK (name <> 'fail_constraint')
        );
        "#
        .to_string(),
    ))
    .await
    .unwrap();

    db.execute(Statement::from_string(
        db.get_database_backend(),
        r#"
        CREATE UNIQUE INDEX idx_name_is_deleted ON public.test_crud_temp (name, is_deleted);
        "#
        .to_string(),
    ))
    .await
    .unwrap();

    db.execute(Statement::from_string(
        db.get_database_backend(),
        r#"
        CREATE UNIQUE INDEX idx_name_active ON public.test_crud_temp (name, active);
        "#
        .to_string(),
    ))
    .await
    .unwrap();

    let res = soft_delete::<Entity, ActiveModel>("non-existent-id", db).await;
    assert!(res.is_err());
    assert!(matches!(res.unwrap_err(), AppError::NotFound(_)));

    let m1 = ActiveModel {
        id: Set("1".to_string()),
        name: Set("dup".to_string()),
        active: Set(true),
        is_deleted: Set(Some(false)),
        deleted_at: Set(None),
        created_at: Set(chrono::Utc::now().into()),
        updated_at: Set(chrono::Utc::now().into()),
    };
    create_record::<Entity, _>(db, m1).await.unwrap();

    let m2 = ActiveModel {
        id: Set("2".to_string()),
        name: Set("dup".to_string()),
        active: Set(false),
        is_deleted: Set(Some(true)),
        deleted_at: Set(Some(chrono::Utc::now().into())),
        created_at: Set(chrono::Utc::now().into()),
        updated_at: Set(chrono::Utc::now().into()),
    };
    m2.insert(db).await.unwrap();

    let res = soft_delete::<Entity, ActiveModel>("1", db).await;
    assert!(res.is_err());
    assert!(matches!(res.unwrap_err(), AppError::Conflict(_)));

    db.execute(Statement::from_string(
        db.get_database_backend(),
        "DELETE FROM public.test_crud_temp;".to_string(),
    ))
    .await
    .unwrap();

    let m3 = ActiveModel {
        id: Set("3".to_string()),
        name: Set("normal".to_string()),
        active: Set(true),
        is_deleted: Set(Some(false)),
        deleted_at: Set(None),
        ..Default::default()
    };
    create_record::<Entity, _>(db, m3).await.unwrap();

    db.execute(Statement::from_string(
        db.get_database_backend(),
        "ALTER TABLE public.test_crud_temp ADD CONSTRAINT chk_deleted_null CHECK (deleted_at IS NULL);"
            .to_string(),
    ))
    .await
    .unwrap();

    let res = soft_delete::<Entity, ActiveModel>("3", db).await;
    assert!(res.is_err());
    let err = res.unwrap_err();
    assert!(!matches!(err, AppError::NotFound(_)));
    assert!(!matches!(err, AppError::Conflict(_)));

    db.execute(Statement::from_string(
        db.get_database_backend(),
        "ALTER TABLE public.test_crud_temp DROP CONSTRAINT chk_deleted_null;".to_string(),
    ))
    .await
    .unwrap();

    db.execute(Statement::from_string(
        db.get_database_backend(),
        "DROP INDEX IF EXISTS idx_name_is_deleted;".to_string(),
    ))
    .await
    .unwrap();

    db.execute(Statement::from_string(
        db.get_database_backend(),
        "DELETE FROM public.test_crud_temp;".to_string(),
    ))
    .await
    .unwrap();

    let res = toggle_status::<Entity, ActiveModel>("non-existent-id", true, db).await;
    assert!(res.is_err());
    assert!(matches!(res.unwrap_err(), AppError::NotFound(_)));

    let m4 = ActiveModel {
        id: Set("4".to_string()),
        name: Set("dup_toggle".to_string()),
        active: Set(true),
        is_deleted: Set(Some(false)),
        deleted_at: Set(None),
        ..Default::default()
    };
    create_record::<Entity, _>(db, m4).await.unwrap();

    let m5 = ActiveModel {
        id: Set("5".to_string()),
        name: Set("dup_toggle".to_string()),
        active: Set(false),
        is_deleted: Set(Some(false)),
        deleted_at: Set(None),
        created_at: Set(chrono::Utc::now().into()),
        updated_at: Set(chrono::Utc::now().into()),
    };
    m5.insert(db).await.unwrap();

    let res = toggle_status::<Entity, ActiveModel>("4", false, db).await;
    assert!(res.is_err());
    assert!(matches!(res.unwrap_err(), AppError::Conflict(_)));

    db.execute(Statement::from_string(
        db.get_database_backend(),
        "DELETE FROM public.test_crud_temp;".to_string(),
    ))
    .await
    .unwrap();

    db.execute(Statement::from_string(
        db.get_database_backend(),
        "ALTER TABLE public.test_crud_temp ADD CONSTRAINT chk_active_true CHECK (active = true);"
            .to_string(),
    ))
    .await
    .unwrap();

    let m5_temp = ActiveModel {
        id: Set("5".to_string()),
        name: Set("active_temp".to_string()),
        active: Set(true),
        is_deleted: Set(Some(false)),
        deleted_at: Set(None),
        ..Default::default()
    };
    create_record::<Entity, _>(db, m5_temp).await.unwrap();

    let res = toggle_status::<Entity, ActiveModel>("5", false, db).await;
    assert!(res.is_err());
    let err = res.unwrap_err();
    assert!(!matches!(err, AppError::NotFound(_)));
    assert!(!matches!(err, AppError::Conflict(_)));

    db.execute(Statement::from_string(
        db.get_database_backend(),
        "ALTER TABLE public.test_crud_temp DROP CONSTRAINT chk_active_true;".to_string(),
    ))
    .await
    .unwrap();

    let m6 = ActiveModel {
        id: Set("6".to_string()),
        name: Set("fail_constraint".to_string()),
        active: Set(true),
        is_deleted: Set(Some(false)),
        deleted_at: Set(None),
        ..Default::default()
    };
    let res = create_record::<Entity, _>(db, m6).await;
    assert!(res.is_err());
    assert!(!matches!(res.unwrap_err(), AppError::Conflict(_)));

    let m7 = ActiveModel {
        id: Set("7".to_string()),
        name: Set("valid_before".to_string()),
        active: Set(true),
        is_deleted: Set(Some(false)),
        deleted_at: Set(None),
        ..Default::default()
    };
    let record7 = create_record::<Entity, _>(db, m7).await.unwrap();
    let mut active7 = record7.into_active_model();
    active7.name = Set("fail_constraint".to_string());

    let res = update_record::<Entity, _>(db, active7).await;
    assert!(res.is_err());
    let err = res.unwrap_err();
    assert!(!matches!(err, AppError::NotFound(_)));
    assert!(!matches!(err, AppError::Conflict(_)));

    let _active_col = Entity::active_column();

    db.execute(Statement::from_string(
        db.get_database_backend(),
        "DROP TABLE IF EXISTS public.test_crud_temp CASCADE;".to_string(),
    ))
    .await
    .unwrap();

    println!("=== CRUD Errors Coverage Tests Passed ===");
}
