use sqlx::SqlitePool;
mod error;
mod quadlet;
mod quadlet_type;
mod response;
mod token_claims;
mod user;

pub use quadlet::{get_quadlet_dir, Quadlet, QuadletInfo, QuadletStatus};
pub use quadlet_type::QuadletType;
pub use response::CustomResponse;
pub use token_claims::TokenClaims;
pub use user::{NewUser, User, UserPass};

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub secret: String,
    pub static_dir: String,
}
