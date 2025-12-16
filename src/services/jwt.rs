use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey};
use serde::{Deserialize, Serialize};
use mongodb::bson::oid::ObjectId;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,  // User ID
    pub mobile: String,
    pub exp: i64,
    pub iat: i64,
}

pub struct JwtService;

impl JwtService {
    pub fn generate_access_token(user_id: &ObjectId, mobile: &str) -> Result<String, jsonwebtoken::errors::Error> {
        let expiry = crate::config::Config::jwt_expiry();
        let now = chrono::Utc::now().timestamp();
        
        let claims = Claims {
            sub: user_id.to_hex(),
            mobile: mobile.to_string(),
            exp: now + expiry,
            iat: now,
        };

        let secret = crate::config::Config::jwt_secret();
        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
    }

    pub fn generate_refresh_token(user_id: &ObjectId, mobile: &str) -> Result<String, jsonwebtoken::errors::Error> {
        let expiry = crate::config::Config::jwt_refresh_expiry();
        let now = chrono::Utc::now().timestamp();
        
        let claims = Claims {
            sub: user_id.to_hex(),
            mobile: mobile.to_string(),
            exp: now + expiry,
            iat: now,
        };

        let secret = crate::config::Config::jwt_refresh_secret();
        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
    }

    pub fn verify_token(token: &str, is_refresh: bool) -> Result<Claims, jsonwebtoken::errors::Error> {
        let secret = if is_refresh {
            crate::config::Config::jwt_refresh_secret()
        } else {
            crate::config::Config::jwt_secret()
        };

        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &Validation::default(),
        )?;

        Ok(token_data.claims)
    }
}
