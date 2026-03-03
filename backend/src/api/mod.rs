mod auth;
mod config;
mod git;
mod health;
mod quadlet;

use crate::models::CustomResponse;
use axum::{http::StatusCode, response::IntoResponse};

pub use auth::router as auth_router;
pub use config::router as config_router;
pub use git::router as git_router;
pub use health::router as health_router;
pub use quadlet::router as quadlet_router;

pub async fn fallback_404() -> impl IntoResponse {
    CustomResponse::<()>::empty(StatusCode::NOT_FOUND, "Not found")
}
