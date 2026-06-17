use crate::{
    config::AppConfig,
    errors::AppError,
    infra::auth::{AuthService, Claims},
    infra::cache::Cache,
};
use axum::{
    extract::{Request, State},
    http::header,
    middleware::Next,
    response::Response,
};

#[derive(Clone, Debug)]
pub struct CurrentUser {
    pub id: String,
    pub email: String,
    pub role: String,
}

pub async fn auth_middleware(
    State((cache, config)): State<(Cache, AppConfig)>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|val| val.to_str().ok())
        .ok_or_else(|| AppError::Unauthorized("Cabeçalho de autorização ausente".to_string()))?;

    if !auth_header.starts_with("Bearer ") {
        return Err(AppError::Unauthorized(
            "Token deve ser do tipo Bearer".to_string(),
        ));
    }

    let token = &auth_header[7..];

    let claims: Claims = AuthService::verify_token(token, &config.jwt_secret)?;

    let is_valid = cache
        .validate_session(&claims.sub, &format!("access:{}", token))
        .await?;
    if !is_valid {
        return Err(AppError::Unauthorized(
            "Sessão revogada ou expirada".to_string(),
        ));
    }

    let current_user = CurrentUser {
        id: claims.sub,
        email: claims.email,
        role: claims.role,
    };
    req.extensions_mut().insert(current_user.clone());

    let mut res = next.run(req).await;
    res.extensions_mut().insert(current_user);
    Ok(res)
}
