use crate::errors::AppError;
use bcrypt::{hash, verify};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

const BCRYPT_COST: u32 = 12;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub email: String,
    pub role: String,
    pub exp: i64,
    pub iat: i64,
}

pub struct AuthService;

impl AuthService {
    pub fn hash_password(password: &str) -> Result<String, AppError> {
        let hashed = hash(password, BCRYPT_COST)
            .map_err(|e| AppError::Internal(format!("Falha ao criptografar senha: {}", e)))?;
        Ok(hashed)
    }

    pub fn verify_password(password: &str, hash: &str) -> Result<bool, AppError> {
        let matches = verify(password, hash)
            .map_err(|e| AppError::Internal(format!("Falha ao verificar senha: {}", e)))?;
        Ok(matches)
    }

    pub fn generate_tokens(
        user_id: &str,
        email: &str,
        role: &str,
        secret: &str,
        expires_sec: i64,
    ) -> Result<(String, String), AppError> {
        let now = Utc::now();
        let iat = now.timestamp();
        let exp = now + Duration::seconds(expires_sec);

        let claims = Claims {
            sub: user_id.to_string(),
            email: email.to_string(),
            role: role.to_string(),
            exp: exp.timestamp(),
            iat,
        };

        let access_token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .map_err(|e| AppError::Internal(format!("Erro ao assinar JWT: {}", e)))?;

        let refresh_claims = Claims {
            sub: user_id.to_string(),
            email: email.to_string(),
            role: role.to_string(),
            exp: (now + Duration::seconds(7 * 24 * 60 * 60)).timestamp(),
            iat,
        };

        let refresh_token = encode(
            &Header::default(),
            &refresh_claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .map_err(|e| AppError::Internal(format!("Erro ao assinar Refresh JWT: {}", e)))?;

        Ok((access_token, refresh_token))
    }

    pub fn verify_token(token: &str, secret: &str) -> Result<Claims, AppError> {
        let validation = Validation::default();
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &validation,
        )?;
        Ok(token_data.claims)
    }
}
