use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

// JWT secret - in production, load this from environment variables or secure storage
const JWT_SECRET: &str = "your-secret-key-change-this-in-production-use-env-var";
const JWT_EXPIRATION_HOURS: u64 = 24 * 7; // 7 days

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,           // Subject (user email)
    pub user_id: String,       // MongoDB ObjectId as string
    pub exp: u64,              // Expiration time (Unix timestamp)
    pub iat: u64,              // Issued at (Unix timestamp)
}

impl Claims {
    pub fn new(email: String, user_id: String) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();
        
        Claims {
            sub: email,
            user_id,
            exp: now + (JWT_EXPIRATION_HOURS * 3600),
            iat: now,
        }
    }
}

/// Create a JWT token for the authenticated user
pub fn create_jwt(email: String, user_id: String) -> Result<String, jsonwebtoken::errors::Error> {
    let claims = Claims::new(email, user_id);
    let header = Header::new(Algorithm::HS256);
    let encoding_key = EncodingKey::from_secret(JWT_SECRET.as_bytes());
    
    encode(&header, &claims, &encoding_key)
}

/// Validate and decode a JWT token
pub fn validate_jwt(token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let decoding_key = DecodingKey::from_secret(JWT_SECRET.as_bytes());
    let validation = Validation::new(Algorithm::HS256);
    
    let token_data = decode::<Claims>(token, &decoding_key, &validation)?;
    Ok(token_data.claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwt_creation_and_validation() {
        let email = "test@example.com".to_string();
        let user_id = "507f1f77bcf86cd799439011".to_string();
        
        let token = create_jwt(email.clone(), user_id.clone()).unwrap();
        let claims = validate_jwt(&token).unwrap();
        
        assert_eq!(claims.sub, email);
        assert_eq!(claims.user_id, user_id);
        assert!(claims.exp > claims.iat);
    }
}
