use chrono::Utc;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::error::AppError;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // user_id
    pub exp: i64,    // expiry timestamp
    pub iat: i64,    // issued at
}

pub fn create_token(user_id: &str, secret: &str, session_days: i64) -> Result<String, AppError> {
    let now = Utc::now().timestamp();
    let expiry = now + (session_days * 24 * 60 * 60);

    let claims = Claims {
        sub: user_id.to_string(),
        exp: expiry,
        iat: now,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )?;

    Ok(token)
}

pub fn verify_token(token: &str, secret: &str) -> Result<Claims, AppError> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )?;

    Ok(token_data.claims)
}
