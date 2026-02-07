use crate::models::{AppState, CustomResponse};
use axum::{http::StatusCode, response::IntoResponse, routing, Router};
use std::sync::Arc;

pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/", routing::get(check_health))
}

async fn check_health() -> impl IntoResponse {
    CustomResponse::<()>::empty(StatusCode::OK, "🚀 Up and running")
}
