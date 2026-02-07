use axum::{
    http::StatusCode,
    extract::{Query, Path},
    Json, response::IntoResponse, routing, Router};
use serde::Deserialize;
use std::sync::Arc;
use crate::system;
use crate::models::{AppState, Quadlet, QuadletType, CustomResponse};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/{extension}", routing::get(read_quadlets))
        .route("/{extension}/{name}", routing::get(read_quadlet))
        .route("/{extension}/{name}", routing::post(save_quadlet))
        .route("/{extension}/{name}", routing::delete(delete_quadlet))
        .route("/{extension}/{name}/action", routing::post(run_action))
        .route("/{extension}/{name}/logs", routing::get(get_quadlet_logs))
}

async fn read_quadlets(Path(extension): Path<String>) -> impl IntoResponse {
    match Quadlet::read_by_extension(&extension).await {
        Ok(quadlets) => CustomResponse::api(StatusCode::OK, "quadlets", quadlets),
        Err(e) => CustomResponse::empty(StatusCode::NOT_FOUND, &format!("Error: {}", e)),
    }
}

async fn read_quadlet(Path((extension, name)): Path<(String, String)>) -> impl IntoResponse {
    let mut quadlet = match Quadlet::new(&name, &extension, None) {
        Ok(quadlet) => quadlet,
        Err(e) => return CustomResponse::empty(StatusCode::BAD_REQUEST, &format!("Invalid quadlet type: {}. {}", extension, e)),
    };
    match quadlet.read().await {
        Ok(_) => CustomResponse::api(StatusCode::OK, "quadlet", quadlet),
        Err(e) => CustomResponse::empty(StatusCode::NOT_FOUND, &format!("Error: {}", e)),
    }
}

async fn save_quadlet(
    Path((extension, name)): Path<(String, String)>,
    Json(content): Json<String>,
) -> impl IntoResponse {
    let quadlet = match Quadlet::new(&name, &extension, Some(content)) {
        Ok(quadlet) => quadlet,
        Err(e) => return CustomResponse::empty(StatusCode::BAD_REQUEST, &format!("Error creating quadlet {}.{}: {}", name, extension, e)),
    };
    // 1. Guardar en disco
    if let Err(e) = quadlet.save().await {
        return CustomResponse::empty(StatusCode::INTERNAL_SERVER_ERROR, &format!("Error saving quadlet {}.{}: {}", name, extension, e));
    }

    // 2. Avisar a systemd que hay archivos nuevos (daemon-reload)
    // Usamos la acción que definimos en el paso anterior
    if let Err(e) = system::run_unit_action(&name, "daemon-reload").await {
        return CustomResponse::empty(StatusCode::INTERNAL_SERVER_ERROR, &format!("Saved, but error with daemon reload: {}", e));
    }
    CustomResponse::api(StatusCode::OK, "saved", quadlet)
}

async fn delete_quadlet(
    Path((extension, name)): Path<(String, String)>,
) -> impl IntoResponse {
    let quadlet = Quadlet::new(&name, &extension, None).unwrap();
    match quadlet.delete().await {
        Ok(_) => CustomResponse::api(StatusCode::OK, "deleted", quadlet),
    Err(e) =>
        CustomResponse::empty(StatusCode::INTERNAL_SERVER_ERROR, &format!("Error deleting quadlet {}.{}: {}", name, extension, e)),
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

