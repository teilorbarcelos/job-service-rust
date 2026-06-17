use crate::{
    core::query_parser::{PaginatedResponse, ParsedFilters},
    errors::AppError,
    models::product,
    modules::product::schemas::{CreateProductRequest, ProductResponse, UpdateProductRequest},
};
use sea_orm::{DatabaseConnection, Set};

pub struct ProductModuleService;

impl ProductModuleService {
    pub async fn list_products(
        filters: ParsedFilters,
        db: &DatabaseConnection,
    ) -> Result<PaginatedResponse<ProductResponse>, AppError> {
        crate::core::crud::list_records::<product::Entity, ProductResponse, _>(
            filters,
            db,
            ProductResponse::from,
        )
        .await
    }

    pub async fn get_product_by_id(
        id: &str,
        db: &DatabaseConnection,
    ) -> Result<ProductResponse, AppError> {
        let p = crate::core::crud::get_by_id::<product::Entity>(id, db).await?;
        Ok(ProductResponse::from(p))
    }

    pub async fn create_product(
        payload: CreateProductRequest,
        user_id: &str,
        db: &DatabaseConnection,
    ) -> Result<ProductResponse, AppError> {
        let active_prod = product::ActiveModel {
            name: Set(payload.name),
            sku: Set(payload.sku),
            category: Set(payload.category),
            price: Set(payload.price),
            stock: Set(payload.stock),
            description: Set(payload.description),
            id_user: Set(Some(user_id.to_string())),
            ..Default::default()
        };

        let p = crate::core::crud::create_record::<product::Entity, _>(db, active_prod).await?;

        Ok(ProductResponse::from(p))
    }

    pub async fn update_product(
        id: &str,
        payload: UpdateProductRequest,
        db: &DatabaseConnection,
    ) -> Result<ProductResponse, AppError> {
        let mut active_prod = product::ActiveModel {
            id: Set(id.to_string()),
            name: Set(payload.name),
            sku: Set(payload.sku),
            category: Set(payload.category),
            price: Set(payload.price),
            stock: Set(payload.stock),
            description: Set(payload.description),
            ..Default::default()
        };

        if let Some(act) = payload.active {
            active_prod.active = Set(act);
        }

        let updated =
            crate::core::crud::update_record::<product::Entity, _>(db, active_prod).await?;

        Ok(ProductResponse::from(updated))
    }

    pub async fn delete_product(id: &str, db: &DatabaseConnection) -> Result<(), AppError> {
        crate::core::crud::soft_delete::<product::Entity, product::ActiveModel>(id, db).await
    }

    pub async fn toggle_product_status(
        id: &str,
        active: bool,
        db: &DatabaseConnection,
    ) -> Result<ProductResponse, AppError> {
        let updated = crate::core::crud::toggle_status::<product::Entity, product::ActiveModel>(
            id, active, db,
        )
        .await?;
        Ok(ProductResponse::from(updated))
    }
}
