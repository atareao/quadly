use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing, Json, Router,
};
use tracing::{debug, error};

use crate::models::{AppState, Config, CustomResponse, NewConfig, UpdateConfig};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", routing::get(get_all_configs).post(create_config))
        .route(
            "/{key}",
            routing::get(get_config)
                .put(update_config)
                .delete(delete_config),
        )
        .route(
            "/quadly-templates-url",
            routing::get(get_quadly_templates_url),
        )
}

/// Obtiene todas las configuraciones
async fn get_all_configs(State(app_state): State<Arc<AppState>>) -> impl IntoResponse {
    debug!("Obteniendo todas las configuraciones");

    match Config::get_all(&app_state.pool).await {
        Ok(configs) => CustomResponse::api(StatusCode::OK, "configs", configs),
        Err(e) => {
            error!("Error obteniendo configuraciones: {}", e);
            CustomResponse::empty(StatusCode::INTERNAL_SERVER_ERROR, &format!("Error: {}", e))
        }
    }
}

/// Obtiene una configuración específica por su clave
async fn get_config(
    State(app_state): State<Arc<AppState>>,
    Path(key): Path<String>,
) -> impl IntoResponse {
    debug!("Obteniendo configuración para clave: {}", key);

    match Config::get_by_key(&app_state.pool, &key).await {
        Ok(Some(config)) => CustomResponse::api(StatusCode::OK, "config", config),
        Ok(None) => CustomResponse::empty(
            StatusCode::NOT_FOUND,
            &format!("Configuración no encontrada para la clave: {}", key),
        ),
        Err(e) => {
            error!("Error obteniendo configuración: {}", e);
            CustomResponse::empty(StatusCode::INTERNAL_SERVER_ERROR, &format!("Error: {}", e))
        }
    }
}

/// Crea una nueva configuración
async fn create_config(
    State(app_state): State<Arc<AppState>>,
    Json(new_config): Json<NewConfig>,
) -> impl IntoResponse {
    debug!("Creando nueva configuración: {:?}", new_config);

    match Config::create(&app_state.pool, new_config).await {
        Ok(config) => CustomResponse::api(StatusCode::CREATED, "config", config),
        Err(e) => {
            error!("Error creando configuración: {}", e);
            let error_msg = if e.to_string().contains("UNIQUE constraint failed") {
                "La clave de configuración ya existe"
            } else {
                "Error al crear la configuración"
            };
            CustomResponse::empty(StatusCode::BAD_REQUEST, error_msg)
        }
    }
}

/// Actualiza una configuración existente
async fn update_config(
    State(app_state): State<Arc<AppState>>,
    Path(key): Path<String>,
    Json(update_config): Json<UpdateConfig>,
) -> impl IntoResponse {
    debug!(
        "Actualizando configuración para clave: {} - {:?}",
        key, update_config
    );

    match Config::update_by_key(&app_state.pool, &key, update_config).await {
        Ok(Some(config)) => CustomResponse::api(StatusCode::OK, "config", config),
        Ok(None) => CustomResponse::empty(
            StatusCode::NOT_FOUND,
            &format!("Configuración no encontrada para la clave: {}", key),
        ),
        Err(e) => {
            error!("Error actualizando configuración: {}", e);
            CustomResponse::empty(StatusCode::INTERNAL_SERVER_ERROR, &format!("Error: {}", e))
        }
    }
}

/// Elimina una configuración
async fn delete_config(
    State(app_state): State<Arc<AppState>>,
    Path(key): Path<String>,
) -> impl IntoResponse {
    debug!("Eliminando configuración para clave: {}", key);

    match Config::delete_by_key(&app_state.pool, &key).await {
        Ok(rows_affected) if rows_affected > 0 => {
            CustomResponse::<()>::empty(StatusCode::OK, "Configuración eliminada correctamente")
        }
        Ok(_) => CustomResponse::<()>::empty(
            StatusCode::NOT_FOUND,
            &format!("Configuración no encontrada para la clave: {}", key),
        ),
        Err(e) => {
            error!("Error eliminando configuración: {}", e);
            CustomResponse::empty(StatusCode::INTERNAL_SERVER_ERROR, &format!("Error: {}", e))
        }
    }
}

/// Obtiene específicamente la URL de templates de quadlets
async fn get_quadly_templates_url(State(app_state): State<Arc<AppState>>) -> impl IntoResponse {
    debug!("Obteniendo URL de templates de quadlets");

    match Config::get_quadly_templates_url(&app_state.pool).await {
        Ok(Some(url)) => {
            let response = serde_json::json!({"url": url});
            CustomResponse::api(StatusCode::OK, "quadly_templates_url", response)
        }
        Ok(None) => CustomResponse::empty(
            StatusCode::NOT_FOUND,
            "URL de templates de quadlets no configurada",
        ),
        Err(e) => {
            error!("Error obteniendo URL de templates: {}", e);
            CustomResponse::empty(StatusCode::INTERNAL_SERVER_ERROR, &format!("Error: {}", e))
        }
    }
}
