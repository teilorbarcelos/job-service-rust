use crate::errors::AppError;
use chrono::{NaiveDate, TimeZone, Utc};
use sea_orm::sea_query::{Expr, Func, IntoColumnRef};
use sea_orm::{
    Condition, DatabaseConnection, EntityTrait, FromQueryResult, Order, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Select,
};
use serde::Serialize;
use std::collections::HashMap;

pub fn parse_date(val: &str, end_of_day: bool) -> Result<chrono::DateTime<Utc>, AppError> {
    let parsed = NaiveDate::parse_from_str(val, "%Y-%m-%d").map_err(|_| {
        AppError::BadRequest("Formato de data inválido. Use YYYY-MM-DD.".to_string())
    })?;
    let hms = if end_of_day { (23, 59, 59) } else { (0, 0, 0) };
    let dt = Utc.from_utc_datetime(&parsed.and_hms_opt(hms.0, hms.1, hms.2).unwrap());
    Ok(dt)
}

pub type FilterCallback<E> = dyn Fn(Select<E>, &str) -> Select<E> + Send + Sync;

pub struct FilterDefinition<E: EntityTrait> {
    pub key: String,
    pub apply: Box<FilterCallback<E>>,
}

impl<E: EntityTrait> FilterDefinition<E> {
    pub fn new<F>(key: &str, apply: F) -> Self
    where
        F: Fn(Select<E>, &str) -> Select<E> + Send + Sync + 'static,
    {
        Self {
            key: key.to_string(),
            apply: Box::new(apply),
        }
    }

    pub fn equals<C>(key: &str, column: C) -> Self
    where
        C: IntoColumnRef + Clone + Send + Sync + 'static,
    {
        let col = column.clone();
        Self::new(key, move |q, val| {
            q.filter(Expr::col(col.clone()).eq(val.to_string()))
        })
    }

    pub fn contains<C>(key: &str, column: C) -> Self
    where
        C: IntoColumnRef + Clone + Send + Sync + 'static,
    {
        let col = column.clone();
        Self::new(key, move |q, val| {
            q.filter(
                Expr::expr(Func::lower(Expr::col(col.clone())))
                    .like(format!("%{}%", val.to_lowercase())),
            )
        })
    }

    pub fn boolean<C>(key: &str, column: C) -> Self
    where
        C: IntoColumnRef + Clone + Send + Sync + 'static,
    {
        let col = column.clone();
        Self::new(key, move |q, val| {
            let b = val == "true" || val == "1";
            q.filter(Expr::col(col.clone()).eq(b))
        })
    }

    pub fn date_range<C>(key_prefix: &str, column: C) -> Vec<Self>
    where
        C: IntoColumnRef + Clone + Send + Sync + 'static,
    {
        let col_start = column.clone();
        let col_end = column.clone();
        let key_start = format!("{}_start", key_prefix);
        let key_end = format!("{}_end", key_prefix);

        vec![
            Self::new(&key_start, move |q, val| {
                if let Ok(dt) = parse_date(val, false) {
                    q.filter(Expr::col(col_start.clone()).gte(dt))
                } else {
                    q
                }
            }),
            Self::new(&key_end, move |q, val| {
                if let Ok(dt) = parse_date(val, true) {
                    q.filter(Expr::col(col_end.clone()).lte(dt))
                } else {
                    q
                }
            }),
        ]
    }
}

pub type SearchCallback = dyn Fn(Condition, &str) -> Condition + Send + Sync;

pub struct SearchDefinition {
    pub key: String,
    pub apply: Box<SearchCallback>,
}

impl SearchDefinition {
    pub fn new<F>(key: &str, apply: F) -> Self
    where
        F: Fn(Condition, &str) -> Condition + Send + Sync + 'static,
    {
        Self {
            key: key.to_string(),
            apply: Box::new(apply),
        }
    }

    pub fn contains<C>(key: &str, column: C) -> Self
    where
        C: IntoColumnRef + Clone + Send + Sync + 'static,
    {
        let col = column.clone();
        Self::new(key, move |cond, word| {
            cond.add(
                Expr::expr(Func::lower(Expr::col(col.clone())))
                    .like(format!("%{}%", word.to_lowercase())),
            )
        })
    }
}

pub struct OrderDefinition<E: EntityTrait> {
    pub key: String,
    pub apply: Box<dyn Fn(Select<E>, Order) -> Select<E> + Send + Sync>,
}

impl<E: EntityTrait> OrderDefinition<E> {
    pub fn new<F>(key: &str, apply: F) -> Self
    where
        F: Fn(Select<E>, Order) -> Select<E> + Send + Sync + 'static,
    {
        Self {
            key: key.to_string(),
            apply: Box::new(apply),
        }
    }

    pub fn column<C>(key: &str, column: C) -> Self
    where
        C: IntoColumnRef + Clone + Send + Sync + 'static,
    {
        let col = column.clone();
        Self::new(key, move |q, order| {
            q.order_by(Expr::col(col.clone()), order)
        })
    }

    pub fn case_insensitive<C>(key: &str, column: C) -> Self
    where
        C: IntoColumnRef + Clone + Send + Sync + 'static,
    {
        let col = column.clone();
        Self::new(key, move |q, order| {
            q.order_by(Expr::expr(Func::lower(Expr::col(col.clone()))), order)
        })
    }
}

#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T> {
    pub items: Vec<T>,
    pub total: u64,
    pub page: u64,
    pub size: u64,
}

pub struct QueryValidator;

impl QueryValidator {
    pub fn validate_and_parse(
        params: &HashMap<String, String>,
        allowed_search_fields: &[&str],
        allowed_filterable_fields: &[&str],
    ) -> Result<ParsedFilters, AppError> {
        let reserved_keys = [
            "page",
            "size",
            "searchWord",
            "searchFields",
            "orderBy",
            "orderDirection",
            "ignoreDefaultFilters",
        ];

        let search_word = params.get("searchWord").cloned().filter(|s| !s.is_empty());
        let search_fields_str = params
            .get("searchFields")
            .cloned()
            .filter(|s| !s.is_empty());

        if search_word.is_some() && search_fields_str.is_none() {
            return Err(AppError::BadRequest(
                "O parâmetro \"searchFields\" é obrigatório quando \"searchWord\" é fornecido."
                    .to_string(),
            ));
        }

        let mut parsed_search_fields = Vec::new();

        if let Some(ref fields_str) = search_fields_str {
            for field in fields_str.split(',') {
                let trimmed = field.trim();
                if !trimmed.is_empty() {
                    if !allowed_search_fields.contains(&trimmed) {
                        return Err(AppError::BadRequest(format!(
                            "O campo '{}' não está disponível para pesquisa global.",
                            trimmed
                        )));
                    }
                    parsed_search_fields.push(trimmed.to_string());
                }
            }
        }

        let mut custom_filters = HashMap::new();
        for (key, val) in params.iter() {
            if val.is_empty() {
                continue;
            }
            if reserved_keys.contains(&key.as_str()) {
                continue;
            }

            let mut is_date = false;
            let mut field_key = key.as_str();
            if key.ends_with("_start") {
                field_key = &key[..key.len() - 6];
                is_date = true;
            } else if key.ends_with("_end") {
                field_key = &key[..key.len() - 4];
                is_date = true;
            }

            let mapped_key = field_key;

            if !allowed_filterable_fields.contains(&mapped_key) {
                return Err(AppError::BadRequest(format!(
                    "O filtro '{}' não é permitido para este recurso.",
                    field_key
                )));
            }

            if is_date
                && (mapped_key == "createdAt" || mapped_key == "updatedAt")
                && parse_date(val, false).is_err()
            {
                return Err(AppError::BadRequest(format!(
                    "Formato de '{}' inválido. Use YYYY-MM-DD.",
                    key
                )));
            }

            custom_filters.insert(key.clone(), val.clone());
        }

        let order_by = params.get("orderBy").cloned().filter(|s| !s.is_empty());
        if let Some(ref field) = order_by {
            let mapped_field = field.as_str();
            if !allowed_filterable_fields.contains(&mapped_field)
                && field != "created_at"
                && field != "updated_at"
            {
                return Err(AppError::BadRequest(format!(
                    "A ordenação pelo campo '{}' não é permitida.",
                    field
                )));
            }
        }

        let page = params
            .get("page")
            .and_then(|p| p.parse::<u64>().ok())
            .unwrap_or(0);
        let size = params
            .get("size")
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(25);

        if size > 100 {
            return Err(AppError::BadRequest(
                "O tamanho máximo da página é 100 itens.".to_string(),
            ));
        }

        let order_direction = params
            .get("orderDirection")
            .cloned()
            .unwrap_or_else(|| "asc".to_string());

        let ignore_default_filters = params
            .get("ignoreDefaultFilters")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);

        if allowed_filterable_fields.contains(&"active")
            && !ignore_default_filters
            && !custom_filters.contains_key("active")
        {
            custom_filters.insert("active".to_string(), "true".to_string());
        }

        Ok(ParsedFilters {
            page,
            size,
            search_word,
            search_fields: parsed_search_fields,
            order_by,
            order_direction,
            ignore_default_filters,
            custom_filters,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ParsedFilters {
    pub page: u64,
    pub size: u64,
    pub search_word: Option<String>,
    pub search_fields: Vec<String>,
    pub order_by: Option<String>,
    pub order_direction: String,
    pub ignore_default_filters: bool,
    pub custom_filters: HashMap<String, String>,
}

impl ParsedFilters {
    pub fn apply_filters<E: EntityTrait>(
        &self,
        mut query: Select<E>,
        defs: &[FilterDefinition<E>],
    ) -> Select<E> {
        for def in defs {
            if let Some(val) = self.custom_filters.get(&def.key) {
                query = (def.apply)(query, val);
            }
        }
        query
    }

    pub fn apply_search<E: EntityTrait>(
        &self,
        mut query: Select<E>,
        defs: &[SearchDefinition],
    ) -> Select<E> {
        if let Some(ref word) = self.search_word {
            let mut or_cond = Condition::any();
            for field in &self.search_fields {
                if let Some(def) = defs.iter().find(|d| &d.key == field) {
                    or_cond = (def.apply)(or_cond, word);
                }
            }
            query = query.filter(or_cond);
        }
        query
    }

    pub fn apply_order<E: EntityTrait, C>(
        &self,
        mut query: Select<E>,
        defs: &[OrderDefinition<E>],
        default_column: C,
    ) -> Select<E>
    where
        C: IntoColumnRef + Clone + Send + Sync + 'static,
    {
        let dir = if self.order_direction.to_lowercase() == "desc" {
            Order::Desc
        } else {
            Order::Asc
        };

        if let Some(ref field) = self.order_by {
            if let Some(def) = defs.iter().find(|d| &d.key == field) {
                query = (def.apply)(query, dir);
            } else {
                query = query.order_by(Expr::col(default_column), dir);
            }
        } else {
            query = query.order_by(Expr::col(default_column), Order::Desc);
        }
        query
    }

    pub async fn paginate<E>(
        &self,
        query: Select<E>,
        db: &DatabaseConnection,
    ) -> Result<(Vec<E::Model>, u64), AppError>
    where
        E: EntityTrait,
        E::Model: FromQueryResult + Sized + Send + Sync + 'static,
    {
        let total = query.clone().paginate(db, 1).num_items().await?;
        let offset = self.page * self.size;
        let records = query.limit(self.size).offset(offset).all(db).await?;
        Ok((records, total))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::product;

    #[test]
    fn test_parse_date() {
        assert!(parse_date("2026-05-22", false).is_ok());
        assert!(parse_date("invalid-date", false).is_err());
    }

    #[test]
    fn test_filter_definitions() {
        let q = product::Entity::find();

        let def_eq = FilterDefinition::<product::Entity>::equals("name", product::Column::Name);
        let _q_eq = (def_eq.apply)(q.clone(), "test");

        let def_contains =
            FilterDefinition::<product::Entity>::contains("name", product::Column::Name);
        let _q_contains = (def_contains.apply)(q.clone(), "test");

        let def_bool =
            FilterDefinition::<product::Entity>::boolean("active", product::Column::Active);
        let _q_bool = (def_bool.apply)(q.clone(), "true");

        let def_dates = FilterDefinition::<product::Entity>::date_range(
            "createdAt",
            product::Column::CreatedAt,
        );
        assert_eq!(def_dates.len(), 2);

        let _q_date_start = (def_dates[0].apply)(q.clone(), "2026-05-22");
        let _q_date_end = (def_dates[1].apply)(q.clone(), "2026-05-22");

        let _q_date_start_err = (def_dates[0].apply)(q.clone(), "invalid");
        let _q_date_end_err = (def_dates[1].apply)(q.clone(), "invalid");
    }

    #[test]
    fn test_search_definition() {
        let cond = Condition::any();
        let def = SearchDefinition::contains("name", product::Column::Name);
        let _cond = (def.apply)(cond, "word");
    }

    #[test]
    fn test_order_definitions() {
        let q = product::Entity::find();

        let def_col = OrderDefinition::<product::Entity>::column("sku", product::Column::Sku);
        let _q_col = (def_col.apply)(q.clone(), Order::Asc);

        let def_case =
            OrderDefinition::<product::Entity>::case_insensitive("name", product::Column::Name);
        let _q_case = (def_case.apply)(q.clone(), Order::Desc);
    }

    #[test]
    fn test_query_validator() {
        let mut params = HashMap::new();

        params.insert("searchWord".to_string(), "test".to_string());
        let res = QueryValidator::validate_and_parse(&params, &["name"], &["name"]);
        assert!(res.is_err());

        params.insert("searchFields".to_string(), "invalid_field".to_string());
        let res = QueryValidator::validate_and_parse(&params, &["name"], &["name"]);
        assert!(res.is_err());

        params.insert("searchFields".to_string(), "name".to_string());
        let res = QueryValidator::validate_and_parse(&params, &["name"], &["name"]);
        assert!(res.is_ok());

        params.insert("empty_param".to_string(), "".to_string());
        let res = QueryValidator::validate_and_parse(&params, &["name"], &["name"]);
        assert!(res.is_ok());
        params.remove("empty_param");

        params.insert("invalid_filter".to_string(), "value".to_string());
        let res = QueryValidator::validate_and_parse(&params, &["name"], &["name"]);
        assert!(res.is_err());
        params.remove("invalid_filter");

        params.insert("createdAt_start".to_string(), "invalid-date".to_string());
        let res = QueryValidator::validate_and_parse(&params, &["name"], &["name", "createdAt"]);
        assert!(res.is_err());
        params.remove("createdAt_start");

        params.insert("orderBy".to_string(), "invalid_order".to_string());
        let res = QueryValidator::validate_and_parse(&params, &["name"], &["name"]);
        assert!(res.is_err());
        params.remove("orderBy");

        params.insert("size".to_string(), "101".to_string());
        let res = QueryValidator::validate_and_parse(&params, &["name"], &["name"]);
        assert!(res.is_err());
    }

    #[test]
    fn test_parsed_filters_apply_order_fallback() {
        let mut params = HashMap::new();
        params.insert("orderBy".to_string(), "unmapped_order_field".to_string());

        let parsed =
            QueryValidator::validate_and_parse(&params, &[], &["unmapped_order_field"]).unwrap();

        let q = product::Entity::find();
        let _q = parsed.apply_order(q, &[], product::Column::CreatedAt);
    }
}
