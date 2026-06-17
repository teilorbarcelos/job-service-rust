use crate::core::query_parser::{
    FilterDefinition, OrderDefinition, PaginatedResponse, ParsedFilters, SearchDefinition,
};
use crate::errors::AppError;
use sea_orm::{
    ActiveModelBehavior, ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait,
    IntoActiveModel, QueryFilter,
};

pub trait CrudEntity: EntityTrait {
    fn is_deleted_column() -> Self::Column;
    fn active_column() -> Self::Column;
    fn not_found_error() -> AppError;
    fn create_conflict_error_message(db_msg: &str) -> String;
    fn update_conflict_error_message(db_msg: &str) -> String;
    fn filter_definitions() -> Vec<FilterDefinition<Self>>;
    fn search_definitions() -> Vec<SearchDefinition>;
    fn order_definitions() -> Vec<OrderDefinition<Self>>;
    fn default_order_column() -> Self::Column;
}

pub trait CrudActiveModel: ActiveModelTrait {
    fn set_id(&mut self, id: String);
    fn set_active(&mut self, active: bool);
    fn set_deleted(&mut self);
    fn init_new(&mut self);
    fn set_updated_at(&mut self);
}

pub async fn get_by_id<E>(id: &str, db: &DatabaseConnection) -> Result<E::Model, AppError>
where
    E: CrudEntity,
    E::PrimaryKey: sea_orm::PrimaryKeyTrait<ValueType = String>,
{
    E::find_by_id(id.to_string())
        .filter(E::is_deleted_column().ne(true))
        .one(db)
        .await?
        .ok_or_else(|| E::not_found_error())
}

pub async fn soft_delete<E, A>(id: &str, db: &DatabaseConnection) -> Result<(), AppError>
where
    E: CrudEntity,
    E::PrimaryKey: sea_orm::PrimaryKeyTrait<ValueType = String>,
    A: CrudActiveModel<Entity = E> + ActiveModelBehavior + Default + Send,
    <E as EntityTrait>::Model: IntoActiveModel<A> + Send + Sync,
{
    let mut active_model = <A as std::default::Default>::default();
    active_model.set_id(id.to_string());
    active_model.set_deleted();
    match active_model.update(db).await {
        Ok(_) => Ok(()),
        Err(err) => {
            match &err {
                sea_orm::DbErr::RecordNotFound(_) | sea_orm::DbErr::RecordNotUpdated => {
                    return Err(E::not_found_error());
                }
                _ => {}
            }
            if let Some(sea_orm::SqlErr::UniqueConstraintViolation(msg)) = err.sql_err() {
                return Err(AppError::Conflict(E::update_conflict_error_message(&msg)));
            }
            Err(err.into())
        }
    }
}

pub async fn toggle_status<E, A>(
    id: &str,
    active: bool,
    db: &DatabaseConnection,
) -> Result<E::Model, AppError>
where
    E: CrudEntity,
    E::PrimaryKey: sea_orm::PrimaryKeyTrait<ValueType = String>,
    A: CrudActiveModel<Entity = E> + ActiveModelBehavior + Default + Send,
    <E as EntityTrait>::Model: IntoActiveModel<A> + Send + Sync,
{
    let mut active_model = <A as std::default::Default>::default();
    active_model.set_id(id.to_string());
    active_model.set_active(active);
    match active_model.update(db).await {
        Ok(updated) => Ok(updated),
        Err(err) => {
            match &err {
                sea_orm::DbErr::RecordNotFound(_) | sea_orm::DbErr::RecordNotUpdated => {
                    return Err(E::not_found_error());
                }
                _ => {}
            }
            if let Some(sea_orm::SqlErr::UniqueConstraintViolation(msg)) = err.sql_err() {
                return Err(AppError::Conflict(E::update_conflict_error_message(&msg)));
            }
            Err(err.into())
        }
    }
}

pub async fn list_records<E, R, F>(
    filters: ParsedFilters,
    db: &DatabaseConnection,
    mapper: F,
) -> Result<PaginatedResponse<R>, AppError>
where
    E: CrudEntity,
    E::Model: sea_orm::FromQueryResult + Sized + Send + Sync + 'static,
    F: Fn(E::Model) -> R,
{
    let query = E::find().filter(E::is_deleted_column().ne(true));
    list_records_with_query(
        filters,
        db,
        query,
        &E::filter_definitions(),
        &E::search_definitions(),
        &E::order_definitions(),
        E::default_order_column(),
        mapper,
    )
    .await
}

pub fn validate_and_parse<E>(
    params: &std::collections::HashMap<String, String>,
) -> Result<ParsedFilters, AppError>
where
    E: CrudEntity,
{
    let allowed_search_fields: Vec<String> = E::search_definitions()
        .iter()
        .map(|d| d.key.clone())
        .collect();

    let mut allowed_filterable_fields: Vec<String> = Vec::new();
    for def in E::filter_definitions() {
        let key = &def.key;
        let base_key = if key.ends_with("_start") {
            &key[..key.len() - 6]
        } else if key.ends_with("_end") {
            &key[..key.len() - 4]
        } else {
            key
        };
        if !allowed_filterable_fields.contains(&base_key.to_string()) {
            allowed_filterable_fields.push(base_key.to_string());
        }
    }

    let allowed_search_refs: Vec<&str> = allowed_search_fields.iter().map(|s| s.as_str()).collect();
    let allowed_filter_refs: Vec<&str> = allowed_filterable_fields
        .iter()
        .map(|s| s.as_str())
        .collect();

    crate::core::query_parser::QueryValidator::validate_and_parse(
        params,
        &allowed_search_refs,
        &allowed_filter_refs,
    )
}

#[allow(clippy::too_many_arguments)]
pub async fn list_records_with_query<E, R, F>(
    filters: ParsedFilters,
    db: &DatabaseConnection,
    mut query: sea_orm::Select<E>,
    filter_defs: &[FilterDefinition<E>],
    search_defs: &[SearchDefinition],
    order_defs: &[OrderDefinition<E>],
    default_order_column: impl sea_orm::sea_query::IntoColumnRef + Clone + Send + Sync + 'static,
    mapper: F,
) -> Result<PaginatedResponse<R>, AppError>
where
    E: CrudEntity,
    E::Model: sea_orm::FromQueryResult + Sized + Send + Sync + 'static,
    F: Fn(E::Model) -> R,
{
    query = filters.apply_search(query, search_defs);
    query = filters.apply_filters(query, filter_defs);
    query = filters.apply_order(query, order_defs, default_order_column);

    let (records, total) = filters.paginate(query, db).await?;
    let items = records.into_iter().map(mapper).collect();

    Ok(PaginatedResponse {
        items,
        total,
        page: filters.page,
        size: filters.size,
    })
}

#[allow(clippy::too_many_arguments)]
pub async fn fetch_all_records_with_query<E>(
    filters: ParsedFilters,
    db: &DatabaseConnection,
    base_query: sea_orm::Select<E>,
    filter_defs: &[FilterDefinition<E>],
    search_defs: &[SearchDefinition],
    order_defs: &[OrderDefinition<E>],
    default_order_column: impl sea_orm::sea_query::IntoColumnRef + Clone + Send + Sync + 'static,
) -> Result<Vec<E::Model>, AppError>
where
    E: CrudEntity,
    E::Model: sea_orm::FromQueryResult + Sized + Send + Sync + 'static,
{
    use sea_orm::{PaginatorTrait, QuerySelect};

    let mut page = 0;
    let size = 100;
    let mut all_records = Vec::new();

    loop {
        let mut page_filters = filters.clone();
        page_filters.page = page;
        page_filters.size = size;

        let mut query = base_query.clone();
        query = page_filters.apply_search(query, search_defs);
        query = page_filters.apply_filters(query, filter_defs);
        query = page_filters.apply_order(query, order_defs, default_order_column.clone());

        let total_items = query.clone().paginate(db, 1).num_items().await?;
        let offset = page * size;
        let page_query = query.limit(size).offset(offset);

        let records = page_query.all(db).await?;
        let len = records.len();
        all_records.extend(records);

        if all_records.len() >= total_items as usize || len < size as usize {
            break;
        }
        page += 1;
    }

    Ok(all_records)
}

pub async fn create_record<E, A>(
    db: &DatabaseConnection,
    mut active_model: A,
) -> Result<E::Model, AppError>
where
    E: CrudEntity,
    E::PrimaryKey: sea_orm::PrimaryKeyTrait<ValueType = String>,
    A: CrudActiveModel<Entity = E> + ActiveModelBehavior + Send,
    <E as EntityTrait>::Model: IntoActiveModel<A> + Send + Sync,
{
    active_model.init_new();
    match active_model.insert(db).await {
        Ok(inserted) => Ok(inserted),
        Err(err) => {
            if let Some(sea_orm::SqlErr::UniqueConstraintViolation(msg)) = err.sql_err() {
                return Err(AppError::Conflict(E::create_conflict_error_message(&msg)));
            }
            Err(err.into())
        }
    }
}

pub async fn update_record<E, A>(
    db: &DatabaseConnection,
    mut active_model: A,
) -> Result<E::Model, AppError>
where
    E: CrudEntity,
    E::PrimaryKey: sea_orm::PrimaryKeyTrait<ValueType = String>,
    A: CrudActiveModel<Entity = E> + ActiveModelBehavior + Send,
    <E as EntityTrait>::Model: IntoActiveModel<A> + Send + Sync,
{
    active_model.set_updated_at();
    match active_model.update(db).await {
        Ok(updated) => Ok(updated),
        Err(err) => {
            match &err {
                sea_orm::DbErr::RecordNotFound(_) | sea_orm::DbErr::RecordNotUpdated => {
                    return Err(E::not_found_error());
                }
                _ => {}
            }
            if let Some(sea_orm::SqlErr::UniqueConstraintViolation(msg)) = err.sql_err() {
                return Err(AppError::Conflict(E::update_conflict_error_message(&msg)));
            }
            Err(err.into())
        }
    }
}

#[macro_export]
macro_rules! impl_crud_traits {
    (
        $entity:path,
        $active_model:path,
        $is_deleted_col:expr,
        $active_col:expr,
        $not_found_msg:expr,
        $create_conflict_msg:expr,
        $update_conflict_msg:expr,
        $filter_defs:expr,
        $search_defs:expr,
        $order_defs:expr,
        $default_order_col:expr
    ) => {
        impl $crate::core::crud::CrudEntity for $entity {
            fn is_deleted_column() -> Self::Column {
                $is_deleted_col
            }
            fn active_column() -> Self::Column {
                $active_col
            }
            fn not_found_error() -> $crate::errors::AppError {
                $crate::errors::AppError::NotFound($not_found_msg.to_string())
            }
            fn create_conflict_error_message(db_msg: &str) -> String {
                let f = $create_conflict_msg;
                f(db_msg)
            }
            fn update_conflict_error_message(db_msg: &str) -> String {
                let f = $update_conflict_msg;
                f(db_msg)
            }
            fn filter_definitions() -> Vec<$crate::core::query_parser::FilterDefinition<Self>> {
                $filter_defs
            }
            fn search_definitions() -> Vec<$crate::core::query_parser::SearchDefinition> {
                $search_defs
            }
            fn order_definitions() -> Vec<$crate::core::query_parser::OrderDefinition<Self>> {
                $order_defs
            }
            fn default_order_column() -> Self::Column {
                $default_order_col
            }
        }

        impl $crate::core::crud::CrudActiveModel for $active_model {
            fn set_id(&mut self, id: String) {
                self.id = sea_orm::Set(id);
            }
            fn set_active(&mut self, active: bool) {
                self.active = sea_orm::Set(active);
                self.updated_at = sea_orm::Set(chrono::Utc::now().into());
            }
            fn set_deleted(&mut self) {
                self.active = sea_orm::Set(false);
                self.is_deleted = sea_orm::Set(Some(true));
                self.deleted_at = sea_orm::Set(Some(chrono::Utc::now().into()));
            }
            fn init_new(&mut self) {
                if !self.id.is_set() {
                    self.id = sea_orm::Set(uuid::Uuid::new_v4().to_string());
                }
                self.active = sea_orm::Set(true);
                self.is_deleted = sea_orm::Set(Some(false));
                self.deleted_at = sea_orm::Set(None);
                self.created_at = sea_orm::Set(chrono::Utc::now().into());
                self.updated_at = sea_orm::Set(chrono::Utc::now().into());
            }
            fn set_updated_at(&mut self) {
                self.updated_at = sea_orm::Set(chrono::Utc::now().into());
            }
        }
    };
}
