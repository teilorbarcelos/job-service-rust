use crate::common::TestContext;
use backend_rust::infra::bootstrap::bootstrap_database;
use backend_rust::models::{auth, feature, role, role_feature, user};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};

pub async fn run(ctx: &TestContext) {
    println!("=== Running Bootstrap Tests ===");

    if let Some(admin_role) = role::Entity::find_by_id("administrator".to_string())
        .one(&ctx.db)
        .await
        .unwrap()
    {
        let mut active_role: role::ActiveModel = admin_role.into();
        active_role.name = Set("Admin Temp".to_string());
        active_role.update(&ctx.db).await.unwrap();
    }

    bootstrap_database(&ctx.db).await.unwrap();

    let admin_role = role::Entity::find_by_id("administrator".to_string())
        .one(&ctx.db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(admin_role.name, "Administrador");

    let db = &ctx.db;

    let _ = role_feature::Entity::delete_many().exec(db).await;
    let _ = user::Entity::delete_many().exec(db).await;
    let _ = auth::Entity::delete_many().exec(db).await;
    let _ = role::Entity::delete_many().exec(db).await;
    let _ = feature::Entity::delete_many().exec(db).await;

    bootstrap_database(db).await.unwrap();

    let features_count = feature::Entity::find().all(db).await.unwrap().len();
    assert!(features_count >= 3);

    let seeded_role = role::Entity::find_by_id("administrator".to_string())
        .one(db)
        .await
        .unwrap();
    assert!(seeded_role.is_some());

    let mapping_count = role_feature::Entity::find().all(db).await.unwrap().len();
    assert!(mapping_count >= 3);

    let seeded_user = user::Entity::find()
        .filter(user::Column::Email.eq("admin@email.com"))
        .one(db)
        .await
        .unwrap();
    assert!(seeded_user.is_some());
    let u = seeded_user.unwrap();
    assert!(u.id_auth.is_some());

    let seeded_auth = auth::Entity::find_by_id(u.id_auth.unwrap())
        .one(db)
        .await
        .unwrap();
    assert!(seeded_auth.is_some());
}
