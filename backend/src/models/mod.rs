use sqlx::SqlitePool;
mod config;
mod error;
mod quadlet;
mod quadlet_type;
mod response;
mod template;
mod token_claims;
mod user;

pub use config::{Config, NewConfig, UpdateConfig};
pub use quadlet::{get_git_repo_dir, get_quadlet_dir, Quadlet, QuadletInfo, QuadletStatus};
pub use quadlet_type::QuadletType;
pub use response::CustomResponse;
pub use template::{
    OutputDestination, ProcessStackRequest, ProcessStackResult, ProcessedTemplate, StackMetadata,
    TemplateStack, TemplateStackInfo,
};
pub use token_claims::TokenClaims;
pub use user::{NewUser, User, UserPass};

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub secret: String,
    pub static_dir: String,
}
