mod auth;
mod quadlet;
mod health;

use crate::models::CustomResponse;
use axum::{http::StatusCode, response::IntoResponse};

pub use quadlet::router as quadlet_router;
pub use health::router as health_router;
pub use auth::router as auth_router;

pub async fn fallback_404() -> impl IntoResponse {
    CustomResponse::<()>::empty( StatusCode::NOT_FOUND, "Not found")
}
