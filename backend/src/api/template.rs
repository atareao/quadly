use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing, Json, Router,
};
use tracing::{debug, error};

use crate::models::{AppState, CustomResponse, ProcessStackRequest, TemplateStack};
use crate::system::TemplateManager;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/sync", routing::post(sync_templates))
        .route("/stacks", routing::get(list_stacks))
        .route("/stacks/{name}", routing::get(get_stack))
        .route("/stacks/{name}/process", routing::post(process_stack))
}

/// Sincroniza (clona/actualiza) el repositorio de plantillas
async fn sync_templates(State(app_state): State<Arc<AppState>>) -> impl IntoResponse {
    debug!("Sincronizando repositorio de plantillas");

    let template_manager = TemplateManager::new();

    match template_manager.sync_templates(&app_state.pool).await {
        Ok(_) => {
            CustomResponse::<()>::empty(StatusCode::OK, "Plantillas sincronizadas correctamente")
        }
        Err(e) => {
            error!("Error sincronizando plantillas: {:#}", e);
            CustomResponse::<()>::empty(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Error sincronizando plantillas: {:#}", e),
            )
        }
    }
}

/// Lista todos los stacks de plantillas disponibles
async fn list_stacks(State(_app_state): State<Arc<AppState>>) -> impl IntoResponse {
    debug!("Listando stacks de plantillas");

    let template_manager = TemplateManager::new();

    match template_manager.list_stacks().await {
        Ok(stacks) => CustomResponse::api(StatusCode::OK, "stacks", stacks),
        Err(e) => {
            error!("Error listando stacks: {}", e);
            CustomResponse::<Vec<TemplateStack>>::empty(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Error: {}", e),
            )
        }
    }
}

/// Obtiene información detallada de un stack específico
async fn get_stack(
    State(_app_state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    debug!("Obteniendo información del stack: {}", name);

    let template_manager = TemplateManager::new();

    match template_manager.get_stack(&name).await {
        Ok(Some(stack)) => CustomResponse::api(StatusCode::OK, "stack", stack),
        Ok(None) => CustomResponse::<TemplateStack>::empty(
            StatusCode::NOT_FOUND,
            &format!("Stack '{}' no encontrado", name),
        ),
        Err(e) => {
            error!("Error obteniendo stack {}: {}", name, e);
            CustomResponse::<TemplateStack>::empty(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Error: {}", e),
            )
        }
    }
}

/// Procesa un stack de plantillas con las variables proporcionadas
async fn process_stack(
    State(app_state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(mut request): Json<ProcessStackRequest>,
) -> impl IntoResponse {
    debug!(
        "Procesando stack: {} con variables: {:?}",
        name, request.variables
    );

    // Asegurar que el nombre del stack coincida con el del path
    request.stack_name = name;

    let template_manager = TemplateManager::new();

    match template_manager
        .process_stack(request, &app_state.pool)
        .await
    {
        Ok(result) => {
            if result.errors.is_empty() {
                CustomResponse::api(StatusCode::OK, "result", result)
            } else {
                debug!("Stack procesado con errores: {:?}", result.errors);
                CustomResponse::api(StatusCode::PARTIAL_CONTENT, "result", result)
            }
        }
        Err(e) => {
            error!("Error procesando stack: {}", e);
            CustomResponse::empty(StatusCode::INTERNAL_SERVER_ERROR, &format!("Error: {}", e))
        }
    }
}
