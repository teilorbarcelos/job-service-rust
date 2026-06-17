#![allow(unexpected_cfgs)]

use axum::{
    extract::rejection::JsonRejection,
    extract::FromRequest,
    extract::Request,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorResponse {
    pub status: bool,
    pub message: String,
    pub error: String,
}

#[derive(Debug)]
pub enum AppError {
    BadRequest(String),
    Unauthorized(String),
    Forbidden(String),
    NotFound(String),
    Conflict(String),
    Internal(String),
}

impl AppError {
    pub fn message(&self) -> String {
        match self {
            AppError::BadRequest(msg) => msg.clone(),
            AppError::Unauthorized(msg) => msg.clone(),
            AppError::Forbidden(msg) => msg.clone(),
            AppError::NotFound(msg) => msg.clone(),
            AppError::Conflict(msg) => msg.clone(),
            AppError::Internal(msg) => msg.clone(),
        }
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status_code, error_name, message) = match self {
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "BadRequestError", msg),
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, "UnauthorizedError", msg),
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, "ForbiddenError", msg),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, "NotFoundError", msg),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, "ConflictError", msg),
            AppError::Internal(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "InternalServerError",
                msg,
            ),
        };

        if status_code == StatusCode::INTERNAL_SERVER_ERROR {
            tracing::error!("Internal AppError: {}", message);
        }

        let body = Json(ErrorResponse {
            status: false,
            message,
            error: error_name.to_string(),
        });

        (status_code, body).into_response()
    }
}

impl From<sea_orm::DbErr> for AppError {
    fn from(err: sea_orm::DbErr) -> Self {
        tracing::error!("Erro de banco de dados detalhado: {}", err);
        AppError::Internal("Erro interno ao processar a requisição".to_string())
    }
}

impl From<bcrypt::BcryptError> for AppError {
    fn from(err: bcrypt::BcryptError) -> Self {
        AppError::Internal(format!("Erro de criptografia: {}", err))
    }
}

impl From<jsonwebtoken::errors::Error> for AppError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        AppError::Unauthorized(format!("Token JWT inválido ou expirado: {}", err))
    }
}

#[cfg(not(tarpaulin_include))]
fn fallback_rejection() -> String {
    "Falha ao desserializar o corpo da requisição".to_string()
}

fn map_rejection(rejection: JsonRejection) -> AppError {
    let msg = match rejection {
        JsonRejection::MissingJsonContentType(_) => "Cabeçalho Content-Type esperado".to_string(),
        JsonRejection::BytesRejection(e) => {
            format!("Falha ao ler o corpo da requisição: {}", e)
        }
        JsonRejection::JsonSyntaxError(e) => format!("Erro de sintaxe no JSON: {}", e),
        JsonRejection::JsonDataError(e) => format!("Erro de validação do JSON: {}", e),
        _ => fallback_rejection(),
    };
    AppError::BadRequest(msg)
}

fn handle_from_request_result<T>(
    res: Result<axum::Json<T>, JsonRejection>,
) -> Result<AppJson<T>, AppError> {
    match res {
        Ok(value) => Ok(AppJson(value.0)),
        Err(rejection) => Err(map_rejection(rejection)),
    }
}

pub struct AppJson<T>(pub T);

impl<S, T> FromRequest<S> for AppJson<T>
where
    T: serde::de::DeserializeOwned + Send + 'static,
    S: Send + Sync + 'static,
{
    type Rejection = AppError;

    fn from_request<'life0, 'async_trait>(
        req: Request,
        state: &'life0 S,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self, Self::Rejection>> + Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        use futures_util::FutureExt;
        Box::pin(axum::Json::<T>::from_request(req, state).map(handle_from_request_result))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Request;

    #[test]
    fn test_app_error_messages_and_responses() {
        let e1 = AppError::BadRequest("bad".to_string());
        assert!(!e1.message().is_empty());
        assert!(!e1.into_response().status().is_success());

        let e2 = AppError::Unauthorized("unauth".to_string());
        assert!(!e2.message().is_empty());
        assert!(!e2.into_response().status().is_success());

        let e3 = AppError::Forbidden("forbid".to_string());
        assert!(!e3.message().is_empty());
        assert!(!e3.into_response().status().is_success());

        let e4 = AppError::NotFound("notfound".to_string());
        assert!(!e4.message().is_empty());
        assert!(!e4.into_response().status().is_success());

        let e5 = AppError::Conflict("conflict".to_string());
        assert!(!e5.message().is_empty());
        assert!(!e5.into_response().status().is_success());

        let e6 = AppError::Internal("internal".to_string());
        assert!(!e6.message().is_empty());
        assert!(!e6.into_response().status().is_success());
    }

    #[test]
    fn test_app_error_display() {
        let err = AppError::BadRequest("test display error".to_string());
        assert_eq!(format!("{}", err), "test display error");
    }

    #[test]
    fn test_from_conversions() {
        let db_err = sea_orm::DbErr::Custom("db error".to_string());
        let app_err = AppError::from(db_err);
        assert!(app_err.message().contains("Erro interno"));

        let bcrypt_err = bcrypt::BcryptError::InvalidCost("1".to_string());
        let app_err = AppError::from(bcrypt_err);
        assert!(app_err.message().contains("Erro de criptografia"));

        let jwt_err =
            jsonwebtoken::errors::Error::from(jsonwebtoken::errors::ErrorKind::ExpiredSignature);
        let app_err = AppError::from(jwt_err);
        assert!(app_err.message().contains("Token JWT inválido ou expirado"));
    }

    #[tokio::test]
    async fn test_app_json_rejections() {
        #[derive(serde::Deserialize)]
        struct Dummy {
            _val: i32,
        }

        let req = Request::builder()
            .method("POST")
            .body(axum::body::Body::from("{\"_val\": 123}"))
            .unwrap();
        let res = AppJson::<Dummy>::from_request(req, &()).await;
        assert!(res.is_err());
        let err = match res {
            Err(e) => e,
            Ok(_) => panic!("expected error"),
        };
        assert!(err.message().contains("Content-Type"));

        let req = Request::builder()
            .method("POST")
            .header("Content-Type", "application/json")
            .body(axum::body::Body::from("{\"_val\":"))
            .unwrap();
        let res = AppJson::<Dummy>::from_request(req, &()).await;
        assert!(res.is_err());
        let err = match res {
            Err(e) => e,
            Ok(_) => panic!("expected error"),
        };
        assert!(err.message().contains("sintaxe"));

        let req = Request::builder()
            .method("POST")
            .header("Content-Type", "application/json")
            .body(axum::body::Body::from("{\"_val\": \"not-an-int\"}"))
            .unwrap();
        let res = AppJson::<Dummy>::from_request(req, &()).await;
        assert!(res.is_err());
        let err = match res {
            Err(e) => e,
            Ok(_) => panic!("expected error"),
        };
        assert!(err.message().contains("validação"));

        let stream = futures_util::stream::once(async {
            let res: Result<axum::body::Bytes, std::io::Error> =
                Err(std::io::Error::other("Forced bytes error"));
            res
        });
        let req = Request::builder()
            .method("POST")
            .header("Content-Type", "application/json")
            .body(axum::body::Body::from_stream(stream))
            .unwrap();
        let res = AppJson::<Dummy>::from_request(req, &()).await;
        assert!(res.is_err());
        let err = match res {
            Err(e) => e,
            Ok(_) => panic!("expected error"),
        };
        assert!(err
            .message()
            .contains("Falha ao ler o corpo da requisição:"));

        let req = Request::builder()
            .method("POST")
            .header("Content-Type", "application/json")
            .body(axum::body::Body::from("{\"_val\": 42}"))
            .unwrap();
        let res = AppJson::<Dummy>::from_request(req, &()).await;
        assert!(res.is_ok());
        assert_eq!(res.unwrap().0._val, 42);
    }
}
