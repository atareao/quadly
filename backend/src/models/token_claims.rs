use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenClaims {
    pub sub: String,
    pub role: String,
    pub iat: usize,
    pub exp: usize,
}
