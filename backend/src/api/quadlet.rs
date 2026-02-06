use crate::models::{Quadlet, CustomResponse};
use crate::system;
use axum::{
    http::StatusCode,
    extract::{Query, Path},
    Json, response::IntoResponse, routing, Router};
use futures::future::join_all;
use serde::Deserialize;
use std::sync::Arc;
use crate::models::AppState;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/quadlets/:type", routing::get(read_quadlets))
        .route("/quadlets/:type/:name", routing::get(read_quadlet))
        .route("/quadlets/:type/:name", routing::post(save_quadlet))
        .route("/quadlets/:type/:name", routing::delete(delete_quadlet))
        .route("/quadlets/:type/:name/action", routing::post(run_action))
        .route("/quadlets/:type/:name/logs", routing::get(get_quadlet_logs))
}

async fn read_quadlets(Path(type: String): Path<String>) -> impl IntoResponse {
    match Quadlet::read_by_type(type).await {
        Ok(quadlets) => CustomResponse::api(StatusCode::OK, "result", quadlets)
        Err(e) => CustomResponse::empty(StatusCode::INTERNAL_SERVER_ERROR, "error")
    }
}

async fn read_quadlet(Path(name): Path<String>) -> impl IntoResponse {
    match Quadlet::read(&name).await {
        Ok(content) => (StatusCode::OK, content).into_response(),
        Err(e) => (StatusCode::NOT_FOUND, e.to_string()).into_response(),
    }
}

#[derive(Deserialize)]
pub struct ActionRequest {
    pub action: String, // "start", "stop", "restart", "daemon-reload"
}

async fn run_action(
    Path(name): Path<String>,
    Json(payload): Json<ActionRequest>,
) -> impl IntoResponse {
    match system::run_unit_action(&name, &payload.action).await {
        Ok(_) => {
            // Si hacemos un cambio de estado, podemos emitir una notificación
            // manual al canal de eventos si quisiéramos respuesta inmediata
            StatusCode::OK
        }
        Err(e) => {
            eprintln!("Error ejecutando {} en {}: {}", payload.action, name, e);
            StatusCode::INTERNAL_SERVER_ERROR
        }
    }
}

#[derive(Deserialize)]
pub struct SaveRequest {
    pub content: String,
}

async fn save_quadlet(
    Path(name): Path<String>,
    Json(payload): Json<SaveRequest>,
) -> impl IntoResponse {
    // 1. Guardar en disco
    if let Err(e) = system::write_quadlet(&name, &payload.content) {
        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
    }

    // 2. Avisar a systemd que hay archivos nuevos (daemon-reload)
    // Usamos la acción que definimos en el paso anterior
    if let Err(e) = system::run_unit_action(&name, "daemon-reload").await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Guardado, pero error en daemon-reload: {}", e),
        )
            .into_response();
    }

    StatusCode::OK.into_response()
}

#[derive(Deserialize)]
pub struct LogsQuery {
    pub lines: Option<u32>,
}

async fn get_quadlet_logs(
    Path(name): Path<String>,
    Query(params): Query<LogsQuery>,
) -> impl IntoResponse {
    let lines = params.lines.unwrap_or(50); // Por defecto 50 líneas

    match system::get_service_logs(&name, lines) {
        Ok(logs) => (StatusCode::OK, logs).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn delete_quadlet(Path(name): Path<String>) -> impl IntoResponse {
    match system::delete_quadlet(&name) {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
