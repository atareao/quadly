use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use ts_rs::TS;

/// Tipos de destino para las plantillas procesadas
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/OutputDestination.ts")]
pub enum OutputDestination {
    #[serde(rename = "systemd")]
    Systemd,
    #[serde(rename = "config")]
    Config,
}

impl std::str::FromStr for OutputDestination {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "systemd" => Ok(OutputDestination::Systemd),
            "config" => Ok(OutputDestination::Config),
            _ => Err(format!("Tipo de destino desconocido: {}", s)),
        }
    }
}

/// Información del stack desde metadata.toml
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/TemplateStackInfo.ts")]
pub struct TemplateStackInfo {
    pub version: String,
    pub description: String,
}

/// Metadatos completos del archivo metadata.toml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackMetadata {
    pub info: TemplateStackInfo,
    pub outputs: HashMap<String, String>, // plantilla -> destino
}

/// Información completa de un stack de plantillas
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/TemplateStack.ts")]
pub struct TemplateStack {
    pub name: String, // nombre de la carpeta
    pub info: TemplateStackInfo,
    pub templates: Vec<String>,           // archivos .j2
    pub static_files: Vec<String>,        // archivos que se copian sin procesar
    pub outputs: HashMap<String, String>, // plantilla -> destino
}

/// Resultado del procesamiento de una plantilla
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../../frontend/src/bindings/ProcessedTemplate.ts")]
pub struct ProcessedTemplate {
    pub template_name: String,
    pub destination: String,
    pub content: String,
    pub output_path: String, // ruta donde se guardará
}

/// Parámetros para procesar un stack
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(
    export,
    export_to = "../../frontend/src/bindings/ProcessStackRequest.ts"
)]
pub struct ProcessStackRequest {
    pub stack_name: String,
    #[ts(type = "Record<string, any>")]
    pub variables: HashMap<String, serde_json::Value>, // variables para las plantillas
    pub target_directory: Option<String>, // directorio de destino opcional
}

/// Resultado completo del procesamiento de un stack
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(
    export,
    export_to = "../../frontend/src/bindings/ProcessStackResult.ts"
)]
pub struct ProcessStackResult {
    pub stack_name: String,
    pub processed_templates: Vec<ProcessedTemplate>,
    pub copied_files: Vec<String>, // archivos estáticos copiados
    pub errors: Vec<String>,       // errores durante el procesamiento
}
