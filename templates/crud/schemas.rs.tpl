use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
pub struct Create{{EntityName}}Request {
{{FieldsStructCreate}}
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct Update{{EntityName}}Request {
{{FieldsStructUpdate}}
}

#[derive(Debug, Serialize, ToSchema)]
pub struct {{EntityName}}Response {
{{FieldsStructResponse}}
}

#[derive(Debug, Serialize, ToSchema)]
pub struct Paginated{{EntityName}}Response {
    pub items: Vec<{{EntityName}}Response>,
    pub total: u64,
    pub page: u64,
    pub size: u64,
}

impl From<crate::models::{{entity_slug}}::Model> for {{EntityName}}Response {
    fn from(p: crate::models::{{entity_slug}}::Model) -> Self {
        Self {
{{ResponseFromModelMappings}}
        }
    }
}
