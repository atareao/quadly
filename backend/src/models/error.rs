use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use std::fmt;
use ts_rs::TS;

#[derive(Debug, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/AppError.ts")]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    pub status: u16,
}

#[derive(Debug)]
pub enum AppError {
    // Errores de sistema
    SystemdError(String),
    StorageError(String),
    ParseError(String),

    // Errores de API
    NotFound(String),
    BadRequest(String),
    InternalServerError(String),
    Unauthorized,

    // Errores de validación
    ValidationError(String),

    // Error genérico para compatibilidad con anyhow
    Generic(anyhow::Error),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::SystemdError(msg) => write!(f, "Error de systemd: {}", msg),
            AppError::StorageError(msg) => write!(f, "Error de almacenamiento: {}", msg),
            AppError::ParseError(msg) => write!(f, "Error de parseo: {}", msg),
            AppError::NotFound(msg) => write!(f, "No encontrado: {}", msg),
            AppError::BadRequest(msg) => write!(f, "Solicitud incorrecta: {}", msg),
            AppError::InternalServerError(msg) => write!(f, "Error interno: {}", msg),
            AppError::Unauthorized => write!(f, "No autorizado"),
            AppError::ValidationError(msg) => write!(f, "Error de validación: {}", msg),
            AppError::Generic(err) => write!(f, "{}", err),
        }
    }
}

impl std::error::Error for AppError {}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_type, message) = match self {
            AppError::SystemdError(msg) => (StatusCode::SERVICE_UNAVAILABLE, "systemd_error", msg),
            AppError::StorageError(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "storage_error", msg)
            }
            AppError::ParseError(msg) => (StatusCode::BAD_REQUEST, "parse_error", msg),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, "not_found", msg),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "bad_request", msg),
            AppError::InternalServerError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal_server_error",
                msg,
            ),
            AppError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                "No autorizado".to_string(),
            ),
            AppError::ValidationError(msg) => {
                (StatusCode::UNPROCESSABLE_ENTITY, "validation_error", msg)
            }
            AppError::Generic(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "generic_error",
                err.to_string(),
            ),
        };

        let error_response = ErrorResponse {
            error: error_type.to_string(),
            message,
            status: status.as_u16(),
        };

        (status, Json(error_response)).into_response()
    }
}

// Implementaciones para convertir desde otros tipos de error
impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        AppError::Generic(err)
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::StorageError(err.to_string())
    }
}

impl From<zbus::Error> for AppError {
    fn from(err: zbus::Error) -> Self {
        AppError::SystemdError(err.to_string())
    }
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError::StorageError(err.to_string())
    }
}

// Métodos de conveniencia para crear errores específicos
impl AppError {
    pub fn not_found(resource: &str) -> Self {
        AppError::NotFound(format!("Recurso '{}' no encontrado", resource))
    }

    pub fn bad_request(msg: &str) -> Self {
        AppError::BadRequest(msg.to_string())
    }

    pub fn validation_error(field: &str, reason: &str) -> Self {
        AppError::ValidationError(format!("Campo '{}': {}", field, reason))
    }

    pub fn systemd_action_failed(action: &str, unit: &str) -> Self {
        AppError::SystemdError(format!(
            "Error ejecutando '{}' en unidad '{}'",
            action, unit
        ))
    }

    pub fn quadlet_parse_error(filename: &str, details: &str) -> Self {
        AppError::ParseError(format!(
            "Error parseando quadlet '{}': {}",
            filename, details
        ))
    }
}
